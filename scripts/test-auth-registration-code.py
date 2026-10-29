"""Black-box smoke test against an explicitly isolated Auth + Mailpit deployment.

Never prints response bodies, tokens, codes or passwords. Uses only stdlib.
"""
import argparse
import json
import re
import uuid
from urllib.error import HTTPError
from urllib.parse import urlencode
from urllib.request import Request, build_opener, ProxyHandler


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--base-url", required=True)
    parser.add_argument("--mailpit-url", required=True)
    parser.add_argument("--confirm-isolated", action="store_true", required=True)
    args = parser.parse_args()
    opener = build_opener(ProxyHandler({}))

    def call(path, body=None, expected=200, base=None):
        data = None if body is None else json.dumps(body).encode()
        request = Request((base or args.base_url).rstrip("/") + path, data=data,
                          headers={"Content-Type": "application/json"})
        try:
            response = opener.open(request, timeout=15)
        except HTTPError as error:
            response = error
        with response:
            if response.status != expected:
                raise RuntimeError(f"Unexpected status for {path}: {response.status}")
            raw = response.read()
            if not raw or "application/json" not in response.headers.get("Content-Type", ""):
                return None
            return json.loads(raw)

    email = f"risk17-{uuid.uuid4()}@example.invalid"
    password_a, password_b = "AttackerTest123!", "OwnerTest456!"

    def latest_code():
        results = call("/api/v1/search?" + urlencode({"query": f"to:{email}"}),
                       base=args.mailpit_url)
        message = call("/api/v1/message/" + results["messages"][0]["ID"],
                       base=args.mailpit_url)
        if "href=" in message["HTML"]:
            raise RuntimeError("Registration email contains a link")
        return re.search(r"(?<!\d)\d{8}(?!\d)", message["Text"])[0]

    first = call("/auth/register", {"email": email, "password": password_a}, 201)
    assert first["verification_email_sent"]
    code_a = latest_code()
    second = call("/auth/register", {"email": email, "password": password_b}, 201)
    assert second["verification_email_sent"]
    code_b = latest_code()
    call("/auth/verify-email?token=retired-test-token", expected=410)
    call("/auth/verify-email", {"token": "retired-test-token"}, 400)
    call("/auth/verify-email", {"registration_attempt_id": second["registration_attempt_id"],
                                "code": code_b})
    call("/auth/verify-email", {"registration_attempt_id": first["registration_attempt_id"],
                                "code": code_a}, 400)
    login = {"email": email, "password": password_a, "client_type": "desktop"}
    call("/api/v1/auth/login", login, 401)
    login["password"] = password_b
    tokens = call("/api/v1/auth/login", login)
    renewed = call("/api/v1/auth/refresh", {"refresh_token": tokens["refresh_token"]})
    assert renewed["refresh_token"] != tokens["refresh_token"]
    call("/api/v1/auth/logout", {"refresh_token": renewed["refresh_token"]}, 204)
    call("/api/v1/auth/refresh", {"refresh_token": renewed["refresh_token"]}, 401)
    print("PASS: attempt-bound registration, legacy rejection, login, refresh and logout")


if __name__ == "__main__":
    main()
