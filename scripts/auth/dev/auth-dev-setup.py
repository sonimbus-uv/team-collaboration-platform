"""Provision local development material without overwriting existing configuration."""

import base64
import hashlib
import json
import os
from pathlib import Path
import secrets
import subprocess
import sys
import tempfile
from urllib.parse import quote


def write_private(path, content):
    with path.open("x", encoding="utf-8") as output:
        output.write(content)
    path.chmod(0o600)


def main():
    os.umask(0o077)
    project = Path(sys.argv[1]).resolve()
    env_path = project / "services/auth/.env"
    if env_path.exists():
        print("Existing Auth configuration preserved.")
        return
    infra = project / "infrastructure/.env"
    if not infra.is_file():
        raise ValueError("Missing infrastructure/.env; provision the development VM first.")

    # Resolve Compose interpolation without sourcing shell code or logging passwords.
    result = subprocess.run(
        ["docker", "compose", "--env-file", str(infra), "-f",
         str(project / "infrastructure/compose.yaml"), "config", "--format", "json"],
        capture_output=True, text=True, check=False,
    )
    if result.returncode:
        raise ValueError("Cannot resolve Compose configuration; check Docker and infrastructure/.env.")
    services = json.loads(result.stdout)["services"]

    def port(service, target):
        matches = [entry.get("published") for entry in services[service].get("ports", [])
                   if int(entry["target"]) == target]
        if len(matches) != 1 or not matches[0] or not str(matches[0]).isdigit():
            raise ValueError(f"Missing or ambiguous published port for {service}.")
        return str(matches[0])

    def encode(value):
        return quote(str(value), safe="")

    pg = services["postgres"]["environment"]
    redis = services["redis"]["environment"]
    database_url = (f"postgres://{encode(pg['POSTGRES_USER'])}:"
                    f"{encode(pg['POSTGRES_PASSWORD'])}@127.0.0.1:"
                    f"{port('postgres', 5432)}/{encode(pg['POSTGRES_DB'])}")
    redis_url = (f"redis://:{encode(redis['REDIS_PASSWORD'])}"
                 f"@127.0.0.1:{port('redis', 6379)}/0")
    smtp_port = port("mailpit", 1025)
    template = (project / "services/auth/.env.example").read_text(encoding="utf-8")

    root = Path.home() / ".local/share"
    root.mkdir(parents=True, exist_ok=True)
    folder = Path(tempfile.mkdtemp(prefix="sonimbus-auth-dev-", dir=root))
    print(f"Development material directory: {folder}")

    def openssl(*args):
        completed = subprocess.run(["openssl", *args], capture_output=True, check=False)
        if completed.returncode:
            raise ValueError("OpenSSL provisioning failed; inspect the development material directory.")
        return completed.stdout

    openssl("genpkey", "-algorithm", "ED25519", "-out", str(folder / "jwt-private.pem"))
    public_der = openssl("pkey", "-in", str(folder / "jwt-private.pem"), "-pubout", "-outform", "DER")
    if len(public_der) != 44 or not public_der.startswith(bytes.fromhex("302a300506032b6570032100")):
        raise ValueError("Unexpected Ed25519 public key format.")
    kid = "dev-" + secrets.token_hex(8)
    public_x = base64.urlsafe_b64encode(public_der[12:]).decode().rstrip("=")
    write_private(folder / "jwt-public-jwks.json", json.dumps({"keys": [{
        "kty": "OKP", "crv": "Ed25519", "use": "sig", "alg": "EdDSA", "kid": kid, "x": public_x,
    }]}, indent=2) + "\n")

    # A dedicated development CA lets Postman verify the server certificate normally.
    openssl("req", "-x509", "-newkey", "rsa:2048", "-sha256", "-nodes", "-days", "365",
            "-keyout", str(folder / "dev-ca.key"), "-out", str(folder / "dev-ca.crt"),
            "-subj", "/CN=Sonimbus Auth Development CA",
            "-addext", "basicConstraints=critical,CA:TRUE",
            "-addext", "keyUsage=critical,keyCertSign,cRLSign")
    openssl("req", "-new", "-newkey", "rsa:2048", "-nodes",
            "-keyout", str(folder / "internal-tls.key"), "-out", str(folder / "internal-tls.csr"),
            "-subj", "/CN=localhost")
    extensions = folder / "server.ext"
    write_private(extensions, "\n".join([
        "subjectAltName=DNS:localhost,IP:127.0.0.1",
        "basicConstraints=critical,CA:FALSE",
        "keyUsage=critical,digitalSignature,keyEncipherment",
        "extendedKeyUsage=serverAuth", "",
    ]))
    openssl("x509", "-req", "-in", str(folder / "internal-tls.csr"),
            "-CA", str(folder / "dev-ca.crt"), "-CAkey", str(folder / "dev-ca.key"),
            "-CAcreateserial", "-days", "30", "-sha256", "-extfile", str(extensions),
            "-out", str(folder / "internal-tls.crt"))

    credentials = {}
    for name in ("core", "files"):
        token = "svc_" + secrets.token_urlsafe(32)
        write_private(folder / f"{name}.token", token + "\n")
        credentials[name] = {
            "enabled": True, "permissions": ["sessions:check"],
            "active_sha256": hashlib.sha256(token.encode()).hexdigest(), "previous": None,
        }
    write_private(folder / "internal-credentials.json", json.dumps(credentials, indent=2) + "\n")

    values = {
        "SERVER_ADDR": "0.0.0.0:3001", "DATABASE_URL": database_url, "REDIS_URL": redis_url,
        "AUTH_LOGIN_ENABLED": "false", "AUTH_INTERNAL_ENABLED": "true",
        "AUTH_INTERNAL_ADDR": "127.0.0.1:3443", "AUTH_INTERNAL_ALLOWED_CIDRS": "127.0.0.1/32",
        "AUTH_INTERNAL_CREDENTIALS_PATH": str(folder / "internal-credentials.json"),
        "AUTH_INTERNAL_CERT_PATH": str(folder / "internal-tls.crt"),
        "AUTH_INTERNAL_KEY_PATH": str(folder / "internal-tls.key"),
        "AUTH_JWT_PRIVATE_KEY_PATH": str(folder / "jwt-private.pem"),
        "AUTH_JWT_JWKS_PATH": str(folder / "jwt-public-jwks.json"), "AUTH_JWT_ACTIVE_KID": kid,
        "AUTH_JWT_ISSUER": "https://auth.example.invalid",
        "AUTH_JWT_AUDIENCES": "sonimbus-core,sonimbus-files,sonimbus-auth",
        "AUTH_REGISTRATION_CODE_HMAC_SECRET": secrets.token_hex(32),
        "AUTH_RECIPIENT_HMAC_SECRET": secrets.token_hex(32),
        "EMAIL_PROVIDER": "mailpit", "MAILPIT_SMTP_HOST": "127.0.0.1",
        "MAILPIT_SMTP_PORT": smtp_port, "TRUSTED_PROXY_IPS": "172.30.250.2",
        "REQUIRE_TRUSTED_PROXY": "true",
    }
    lines, written = [], set()
    for line in template.splitlines():
        key, separator, _ = line.partition("=")
        if separator and key in values:
            lines.append(f"{key}={json.dumps(values[key])}")
            written.add(key)
        else:
            lines.append(line)
    lines.extend(f"{key}={json.dumps(value)}" for key, value in values.items() if key not in written)

    for path in folder.iterdir():
        path.chmod(0o600)
    # Export only public certificates; private material stays outside the shared repository.
    export = project / "secrets" / folder.name
    export.mkdir(parents=True, exist_ok=False)
    for name in ("dev-ca.crt", "internal-tls.crt"):
        write_private(export / name, (folder / name).read_text())
    write_private(env_path, "\n".join(lines) + "\n")
    print("Auth configuration created; login remains disabled by default.")
    print(f"Import this CA certificate into Postman: {export / 'dev-ca.crt'}")
    print("Service tokens are in core.token and files.token; do not share or commit them.")
    print("Next: bash scripts/auth/dev/start-auth-dev.sh --enable-login")


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, KeyError, TypeError):
        # Parser and subprocess details may contain secrets; keep diagnostics generic.
        print("Setup failed. Check tools, Compose configuration and file permissions. "
              "Existing configuration was not overwritten. Generated material, if any, "
              "remains in the reported directory for inspection.", file=sys.stderr)
        sys.exit(1)
