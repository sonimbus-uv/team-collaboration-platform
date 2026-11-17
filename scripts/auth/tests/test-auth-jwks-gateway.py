"""Exercise the real Nginx files against a disposable public-key HTTP stub.

Requires nginx on PATH (or --nginx). No Auth process, Redis or database needed.
Only generated config copies in a TemporaryDirectory are changed.
"""

import argparse
import importlib.util
import json
import shutil
import socket
import ssl
import subprocess
import tempfile
import threading
import time
from functools import partial
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen

CACHE = "public, max-age=300, must-revalidate"
BODY = json.dumps(
    {
        "keys": [
            {
                "kty": "OKP",
                "crv": "Ed25519",
                "use": "sig",
                "alg": "EdDSA",
                "kid": "test-only",
                "x": "test-public-stub",
            }
        ]
    }
).encode()


class Backend(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path == "/.well-known/jwks.json":
            self.send_response(200)
            self.send_header("Content-Type", "application/json")
            self.send_header("Cache-Control", CACHE)
            self.send_header("Content-Length", str(len(BODY)))
            self.end_headers()
            self.wfile.write(BODY)
        else:
            self.send_response(200)
            self.end_headers()

    def do_HEAD(self):
        self.send_response(200)
        self.send_header("Content-Type", "application/json")
        self.send_header("Cache-Control", CACHE)
        self.end_headers()

    def do_POST(self):
        self.send_response(201)
        self.end_headers()

    def log_message(self, *_):
        pass


def request(port, path, method="GET", data=None, context=None):
    scheme = "https" if context is not None else "http"
    req = Request(f"{scheme}://127.0.0.1:{port}{path}", method=method, data=data)
    try:
        response = urlopen(req, timeout=2, context=context)
    except HTTPError as error:
        response = error
    with response:
        return response.status, response.headers, response.read()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--nginx", default=shutil.which("nginx"))
    parser.add_argument(
        "--tls", action="store_true", help="Exercise the development HTTPS listener"
    )
    args = parser.parse_args()
    if not args.nginx:
        parser.error("nginx is required; install it or supply --nginx")
    root = Path(__file__).resolve().parents[3]
    backend = ThreadingHTTPServer(("127.0.0.1", 0), Backend)
    worker = threading.Thread(target=backend.serve_forever, daemon=True)
    worker.start()
    with socket.socket() as probe:
        probe.bind(("127.0.0.1", 0))
        port = probe.getsockname()[1]
    try:
        with tempfile.TemporaryDirectory(prefix="sonimbus-jwks-nginx-") as temporary:
            directory = Path(temporary)
            (directory / "logs").mkdir()
            (directory / "temp").mkdir()
            for subdir in ["routes", "services"]:
                (directory / subdir).mkdir()
                for source in (root / "gateway/nginx" / subdir).glob("*.conf"):
                    text = source.read_text(encoding="utf-8")
                    text = text.replace(
                        "host.docker.internal:3001", f"127.0.0.1:{backend.server_port}"
                    )
                    text = text.replace("/dev/stdout", "logs/access.log")
                    (directory / subdir / source.name).write_text(
                        text, encoding="utf-8"
                    )
            config = (root / "gateway/nginx/nginx.conf").read_text(encoding="utf-8")
            config = config.replace("listen 80;", f"listen 127.0.0.1:{port};")
            config = config.replace("/dev/null", "logs/error.log")
            config = config.replace("/etc/nginx/", directory.as_posix() + "/")
            context = None
            if args.tls:
                spec = importlib.util.spec_from_file_location(
                    "gateway_tls", root / "scripts/auth/dev/setup-gateway-tls.py"
                )
                provisioning = importlib.util.module_from_spec(spec)
                spec.loader.exec_module(provisioning)
                provisioning.provision(directory / "project", directory / "tls")
                with socket.socket() as probe:
                    probe.bind(("127.0.0.1", 0))
                    port = probe.getsockname()[1]
                listeners = directory / "listeners"
                listeners.mkdir()
                listener = (root / "gateway/nginx/tls/development.conf").read_text(
                    encoding="utf-8"
                )
                listener = listener.replace(
                    "listen 443 ssl;", f"listen 127.0.0.1:{port} ssl;"
                )
                listener = listener.replace("/etc/nginx/tls/", str(directory / "tls") + "/")
                (listeners / "development.conf").write_text(
                    listener, encoding="utf-8"
                )
                context = ssl.create_default_context(
                    cafile=str(directory / "tls/ca.crt")
                )
            fetch = partial(request, context=context)
            config = "pid logs/nginx.pid;\n" + config
            paths = "\n".join(
                f"{kind}_temp_path temp/{kind};"
                for kind in ["client_body", "proxy", "fastcgi", "uwsgi", "scgi"]
            )
            config = config.replace("http {", "http {\n" + paths)
            (directory / "nginx.conf").write_text(config, encoding="utf-8")
            command = [args.nginx, "-p", directory.as_posix() + "/", "-c", "nginx.conf"]
            subprocess.run(command + ["-t"], check=True)
            process = subprocess.Popen(command + ["-g", "daemon off;"])
            try:
                deadline = time.monotonic() + 10
                while True:
                    try:
                        status, _, _ = fetch(port, "/health")
                        assert status == 200
                        break
                    except (URLError, OSError):
                        if time.monotonic() >= deadline:
                            raise
                        time.sleep(0.1)
                if args.tls:
                    for protocol in (
                        ssl.TLSVersion.TLSv1_2, ssl.TLSVersion.TLSv1_3
                    ):
                        context.minimum_version = context.maximum_version = protocol
                        assert fetch(port, "/health")[0] == 200
                    context.minimum_version = ssl.TLSVersion.TLSv1_2
                    context.maximum_version = ssl.TLSVersion.MAXIMUM_SUPPORTED
                    try:
                        request(
                            port, "/health", context=ssl.create_default_context()
                        )
                    except URLError as error:
                        assert isinstance(
                            error.reason, ssl.SSLCertVerificationError
                        )
                    else:
                        raise AssertionError(
                            "An untrusted development CA must be rejected"
                        )
                for _ in range(24):
                    status, headers, body = fetch(port, "/.well-known/jwks.json")
                    assert status == 200 and body == BODY
                    assert headers["Cache-Control"] == CACHE
                    assert headers["Content-Type"] == "application/json"
                status, headers, body = fetch(port, "/.well-known/jwks.json", "HEAD")
                assert status == 200 and not body and headers["Cache-Control"] == CACHE
                statuses = [
                    fetch(port, "/api/v1/auth/register", "POST")[0] for _ in range(16)
                ]
                assert (
                    429 in statuses
                ), "existing registration edge budget must remain enabled"
                assert fetch(port, "/.well-known/jwks.json")[0] == 200
                for operation in [
                    "change-password",
                    "forgot-password",
                    "reset-password",
                ]:
                    path = f"/api/v1/auth/{operation}"
                    assert fetch(port, path, "POST")[0] == 201
                    status, headers, body = fetch(port, path, "POST", b"x" * 5000)
                    assert status == 413
                    assert headers["Content-Type"] == "application/problem+json"
                    assert json.loads(body)["type"] == "/problems/request-too-large"
                    assert headers["Cache-Control"] == "no-store"
                for operation in [
                    "login",
                    "refresh",
                    "logout",
                    "change-password",
                    "forgot-password",
                    "reset-password",
                ]:
                    path = f"/api/v1/auth/{operation}"
                    # Stub 201 proves forwarding, not authentication semantics.
                    assert fetch(port, path, "POST")[0] == 201
                    outcomes = [fetch(port, path, "POST") for _ in range(24)]
                    limited = [result for result in outcomes if result[0] == 429]
                    assert limited, f"missing edge budget for {operation}"
                    assert (
                        json.loads(limited[0][2])["type"]
                        == "/problems/too-many-requests"
                    )
                    assert limited[0][1]["Cache-Control"] == "no-store"
                assert fetch(port, "/.well-known/jwks.json")[0] == 200
                for operation, method, expected in [
                    ("attempts", "POST", 201), ("authorize", "GET", 200),
                    ("callback", "GET", 200), ("exchange", "POST", 201),
                    ("link-attempts", "POST", 201),
                ]:
                    assert fetch(port, f"/api/v1/auth/google/{operation}", method)[0] == expected
                for operation in ("attempts", "exchange", "link-attempts"):
                    status, headers, body = fetch(port, f"/api/v1/auth/google/{operation}", "POST", b"x" * 5000)
                    assert status == 413
                    assert json.loads(body)["type"] == "/problems/request-too-large"
                assert fetch(port, "/api/v1/auth/connections")[0] == 200
                assert fetch(port, "/api/v1/auth/reauthenticate", "POST")[0] == 201
                status, _, body = fetch(port, "/api/v1/auth/reauthenticate", "POST", b"x" * 5000)
                assert status == 413 and json.loads(body)["type"] == "/problems/request-too-large"
                outcomes = [fetch(port, "/api/v1/auth/reauthenticate", "POST") for _ in range(12)]
                limited = [result for result in outcomes if result[0] == 429]
                assert limited and json.loads(limited[0][2])["type"] == "/problems/too-many-requests"
                outcomes = [fetch(port, "/api/v1/auth/google/exchange", "POST") for _ in range(24)]
                limited = [result for result in outcomes if result[0] == 429]
                assert limited and json.loads(limited[0][2])["type"] == "/problems/too-many-requests"
                assert fetch(port, "/api/v1/auth/contact-email")[0] == 200
                for route in ("verification-attempts", "verification-resends", "verify"):
                    path = f"/api/v1/auth/contact-email/{route}"
                    assert fetch(port, path, "POST")[0] == 201
                    status, headers, body = fetch(port, path, "POST", b"x" * 5000)
                    assert status == 413 and json.loads(body)["type"] == "/problems/request-too-large"
                outcomes = [fetch(port, "/api/v1/auth/contact-email/verify", "POST") for _ in range(24)]
                limited = [result for result in outcomes if result[0] == 429]
                assert limited and json.loads(limited[0][2])["type"] == "/problems/too-many-requests"
                assert fetch(port, "/not-a-route")[0] == 404
                for internal in (
                    "/internal/v1/sessions/check",
                    "/internal/v1/sessions/check-batch",
                ):
                    assert fetch(port, internal, "POST")[0] == 404
                print(
                    "PASS: nginx -t, public JWKS GET/HEAD, cache headers, Google routes, isolated budgets and default 404"
                )
            finally:
                subprocess.run(
                    command + ["-s", "quit"], check=False, capture_output=True
                )
                try:
                    process.wait(timeout=10)
                except subprocess.TimeoutExpired:
                    process.terminate()
                    process.wait(timeout=5)
    finally:
        backend.shutdown()
        backend.server_close()
        worker.join(timeout=5)


if __name__ == "__main__":
    main()
