"""Exercise Google login on Windows using a temporary desktop protocol receiver."""

import argparse
import base64
import hashlib
from http.server import BaseHTTPRequestHandler, HTTPServer
import json
import os
from pathlib import Path
import secrets
import shutil
import ssl
import subprocess
import tempfile
import time
from urllib.error import HTTPError
from urllib.parse import parse_qs, urlsplit
from urllib.request import Request, urlopen


BASE_URL = "https://localhost:8443"
PROTOCOL_KEY = r"Software\Classes\sonimbus"


def api(context, path, payload=None, headers=None):
    request = Request(
        BASE_URL + "/api/v1/auth" + path,
        data=json.dumps(payload).encode() if payload is not None else None,
        headers={"Content-Type": "application/json", **(headers or {})},
    )
    try:
        response = urlopen(request, context=context, timeout=20)
    except HTTPError as error:
        response = error
    with response:
        content = response.read(65536)
        return response.status, json.loads(content) if content else None


def require_status(actual, expected, stage):
    status, body = actual
    if status != expected:
        kind = body.get("type", "unknown") if isinstance(body, dict) else "unknown"
        raise RuntimeError(f"{stage}: HTTP {status}, problem {kind}")
    print(f"PASS: {stage} (HTTP {status})", flush=True)
    return body


def callback_server(endpoint):
    class Receiver(BaseHTTPRequestHandler):
        result = None

        def do_POST(self):
            length = int(self.headers.get("Content-Length", "0"))
            if self.path != endpoint or not 0 < length <= 8192:
                self.send_error(400)
                return
            try:
                returned = urlsplit(self.rfile.read(length).decode())
                parameters = parse_qs(returned.query, strict_parsing=True)
                valid = (
                    returned.scheme == "sonimbus"
                    and returned.netloc == "auth"
                    and returned.path == "/google"
                    and not returned.fragment
                    and all(len(value) == 1 for value in parameters.values())
                )
                if not valid:
                    raise ValueError("Invalid desktop return")
            except (ValueError, UnicodeError):
                self.send_error(400)
                return
            self.server.result = {name: value[0] for name, value in parameters.items()}
            self.send_response(204)
            self.end_headers()

        def log_message(self, *_):
            pass

    server = HTTPServer(("127.0.0.1", 0), Receiver)
    server.result = None
    server.timeout = 1
    return server


def install_receiver(directory, server, endpoint):
    import winreg

    try:
        with winreg.OpenKey(winreg.HKEY_CURRENT_USER, PROTOCOL_KEY):
            raise RuntimeError(
                "A sonimbus protocol handler already exists; it was preserved."
            )
    except FileNotFoundError:
        pass
    handler = directory / "receive.ps1"
    handler.write_text(
        'param([string]$ReturnUri)\n'
        '$ErrorActionPreference = "Stop"\n'
        f'$endpoint = "http://127.0.0.1:{server.server_port}{endpoint}"\n'
        'Invoke-WebRequest -UseBasicParsing -Uri $endpoint -Method Post '
        '-ContentType "text/plain" -Body ([System.Text.Encoding]::UTF8.GetBytes($ReturnUri)) '
        '| Out-Null\n',
        encoding="utf-8",
    )
    powershell = shutil.which("powershell.exe")
    if not powershell:
        raise RuntimeError("Windows PowerShell is unavailable")
    with winreg.CreateKey(winreg.HKEY_CURRENT_USER, PROTOCOL_KEY) as key:
        winreg.SetValueEx(key, "", 0, winreg.REG_SZ, "URL:Sonimbus Google login smoke test")
        winreg.SetValueEx(key, "URL Protocol", 0, winreg.REG_SZ, "")
    with winreg.CreateKey(
        winreg.HKEY_CURRENT_USER, PROTOCOL_KEY + r"\shell\open\command"
    ) as key:
        command = f'"{powershell}" -NoProfile -WindowStyle Hidden -File "{handler}" "%1"'
        winreg.SetValueEx(key, "", 0, winreg.REG_SZ, command)


def remove_receiver():
    import winreg

    for suffix in (r"\shell\open\command", r"\shell\open", r"\shell", ""):
        winreg.DeleteKey(winreg.HKEY_CURRENT_USER, PROTOCOL_KEY + suffix)


def exchange_proof(attempt, returned, verifier):
    if returned.get("attempt_id") != attempt["attempt_id"]:
        raise RuntimeError("Desktop return belongs to another attempt")
    if "error" in returned:
        raise RuntimeError("Google authorization was cancelled or failed before exchange")
    return {
        "attempt_id": attempt["attempt_id"],
        "completion_code": returned.get("completion_code", ""),
        "code_verifier": verifier,
    }, {"X-Auth-Attempt-Secret": attempt["attempt_secret"]}


def check_session(context, payload, headers, expected_account=None):
    tokens = require_status(
        api(context, "/google/exchange", payload, headers), 200, "Google exchange"
    )
    refresh = tokens["refresh_token"]
    try:
        if expected_account and tokens["account_id"] != expected_account:
            raise RuntimeError("Google login did not open the linked Sonimbus account")
        connections = require_status(
            api(context, "/connections", headers={
                "Authorization": "Bearer " + tokens["access_token"],
            }),
            200, "Authenticated session",
        )
        if not any(
            connection["provider"] == "google"
            for connection in connections["connections"]
        ):
            raise RuntimeError("Google identity is missing from the account")
        require_status(
            api(context, "/google/exchange", payload, headers),
            401, "Exchange replay rejected",
        )
        renewed = require_status(
            api(context, "/refresh", {"refresh_token": refresh}),
            200, "Refresh rotation",
        )
        refresh = renewed["refresh_token"]
        if (
            renewed["sid"] != tokens["sid"]
            or renewed["account_id"] != tokens["account_id"]
        ):
            raise RuntimeError("Refresh changed the session identity")
    finally:
        require_status(
            api(context, "/logout", {"refresh_token": refresh}),
            204, "Test session logout",
        )
    require_status(
        api(context, "/refresh", {"refresh_token": refresh}),
        401, "Revoked refresh rejected",
    )
    require_status(
        api(context, "/connections", headers={"Authorization": "Bearer " + tokens["access_token"]}),
        401, "Revoked session rejected",
    )


def browser_exchange(context, server, access_token=None):
    verifier = secrets.token_urlsafe(32)
    challenge = base64.urlsafe_b64encode(
        hashlib.sha256(verifier.encode()).digest()
    ).decode().rstrip("=")
    payload = {"redirect_target": "desktop", "code_challenge": challenge}
    if access_token:
        path = "/google/link-attempts"
        headers = {"Authorization": "Bearer " + access_token}
    else:
        path, headers = "/google/attempts", {}
        payload.update(client_type="desktop", device_name="Google login smoke test")
    attempt = require_status(api(context, path, payload, headers), 201, "Create Google attempt")
    authorization = urlsplit(attempt["authorization_url"])
    if (
        authorization.scheme != "https"
        or authorization.netloc != "localhost:8443"
        or authorization.path != "/api/v1/auth/google/authorize"
    ):
        raise RuntimeError("Unexpected authorization origin; check AUTH_GOOGLE_CALLBACK_URL")
    server.result = None
    os.startfile(attempt["authorization_url"])
    print(
        "ACTION: Choose your Google account and accept in the browser. "
        "Allow opening the Sonimbus handler when prompted.",
        flush=True,
    )
    deadline = time.monotonic() + 540
    while server.result is None and time.monotonic() < deadline:
        server.handle_request()
    if server.result is None:
        raise RuntimeError("Timed out waiting for browser authorization")
    return exchange_proof(attempt, server.result, verifier)


def read_local_credentials():
    print("ACTION: Enter your existing Sonimbus credentials in the local form.", flush=True)
    command = r"""
[Console]::OutputEncoding = [System.Text.UTF8Encoding]::new($false)
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms
$form = New-Object System.Windows.Forms.Form
$form.Text = 'Connect Google to Sonimbus'
$form.Width = 440
$form.Height = 245
$form.StartPosition = 'CenterScreen'
$form.TopMost = $true
$emailLabel = New-Object System.Windows.Forms.Label
$emailLabel.Text = 'Sonimbus email'
$emailLabel.SetBounds(20, 20, 380, 20)
$emailInput = New-Object System.Windows.Forms.TextBox
$emailInput.SetBounds(20, 45, 380, 25)
$passwordLabel = New-Object System.Windows.Forms.Label
$passwordLabel.Text = 'Sonimbus password (not Google)'
$passwordLabel.SetBounds(20, 80, 380, 20)
$passwordInput = New-Object System.Windows.Forms.TextBox
$passwordInput.UseSystemPasswordChar = $true
$passwordInput.SetBounds(20, 105, 380, 25)
$confirm = New-Object System.Windows.Forms.Button
$confirm.Text = 'Continue'
$confirm.SetBounds(210, 155, 90, 30)
$confirm.DialogResult = [System.Windows.Forms.DialogResult]::OK
$cancel = New-Object System.Windows.Forms.Button
$cancel.Text = 'Cancel'
$cancel.SetBounds(310, 155, 90, 30)
$cancel.DialogResult = [System.Windows.Forms.DialogResult]::Cancel
$form.Controls.AddRange(@($emailLabel, $emailInput, $passwordLabel, $passwordInput, $confirm, $cancel))
$form.AcceptButton = $confirm
$form.CancelButton = $cancel
try {
    if ($form.ShowDialog() -ne [System.Windows.Forms.DialogResult]::OK) { exit 2 }
    @{email=$emailInput.Text.Trim(); password=$passwordInput.Text} | ConvertTo-Json -Compress
}
finally {
    $passwordInput.Clear()
    $form.Dispose()
}
"""
    result = subprocess.run(
        ["powershell.exe", "-NoProfile", "-Command", command],
        capture_output=True, creationflags=subprocess.CREATE_NO_WINDOW,
    )
    if result.returncode:
        raise RuntimeError("Local credential dialog was cancelled or unavailable")
    credentials = json.loads(result.stdout.decode("utf-8-sig"))
    email = credentials.get("email", "")
    if "\\" in email or email.count("@") != 1 or "." not in email.split("@")[-1]:
        raise RuntimeError("Enter the full Sonimbus email address, without a Windows domain prefix")
    if len(credentials.get("password", "")) > 128:
        raise RuntimeError("Sonimbus passwords must contain at most 128 characters")
    return credentials


def local_password_login(context):
    credentials = read_local_credentials()
    credentials.update(client_type="desktop", device_name="Google linking smoke test")
    try:
        return require_status(api(context, "/login", credentials), 200, "Sonimbus password login")
    finally:
        credentials.clear()


def link_existing_account(context, server):
    tokens = local_password_login(context)
    try:
        payload, headers = browser_exchange(context, server, tokens["access_token"])
        require_status(api(context, "/google/exchange", payload, headers), 204, "Google linked")
        connections = require_status(
            api(context, "/connections", headers={
                "Authorization": "Bearer " + tokens["access_token"],
            }),
            200, "Linked connections",
        )
        if not any(entry["provider"] == "google" for entry in connections["connections"]):
            raise RuntimeError("Google connection was not recorded")
        require_status(
            api(context, "/google/exchange", payload, headers),
            401, "Link exchange replay rejected",
        )
        return tokens["account_id"]
    finally:
        require_status(
            api(context, "/logout", {"refresh_token": tokens["refresh_token"]}),
            204, "Password test session logout",
        )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--link", action="store_true", help="Explicitly link an existing Sonimbus account first")
    arguments = parser.parse_args()
    if os.name != "nt":
        raise RuntimeError("Run this helper on Windows with the HTTPS development tunnel active")
    project = Path(__file__).resolve().parents[3]
    context = ssl.create_default_context(cafile=str(project / "secrets/gateway-dev/ca.crt"))
    endpoint = "/" + secrets.token_urlsafe(32)
    with tempfile.TemporaryDirectory(prefix="sonimbus-google-smoke-") as temporary:
        with callback_server(endpoint) as server:
            install_receiver(Path(temporary), server, endpoint)
            try:
                account_id = link_existing_account(context, server) if arguments.link else None
                if arguments.link:
                    print("ACTION: Google is linked. Authorize again to test a fresh Google login.", flush=True)
                payload, headers = browser_exchange(context, server)
                check_session(context, payload, headers, account_id)
                print(
                    "PASS: Complete Google login, refresh and logout. Tokens were not "
                    "saved; the account remains in the development database.",
                    flush=True,
                )
            finally:
                remove_receiver()


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        # Exception details can contain callback URLs or credentials.
        if isinstance(error, RuntimeError):
            print(str(error), flush=True)
        else:
            print(
                "Google smoke test failed; inspect local connectivity and "
                "configuration without logging tokens.",
                flush=True,
            )
        raise SystemExit(1)
