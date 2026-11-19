"""Provision localhost gateway TLS inside the development VM."""

import fcntl
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def openssl(*arguments):
    return subprocess.run(
        ["openssl", *map(str, arguments)],
        check=True,
        capture_output=True,
    ).stdout


def validate_pair(certificate, key):
    certificate_public = openssl("x509", "-in", certificate, "-pubkey", "-noout")
    key_public = openssl("pkey", "-in", key, "-pubout")
    if certificate_public != key_public:
        raise ValueError("Certificate and private key do not match.")


def provision(project, directory):
    directory.mkdir(parents=True, exist_ok=True, mode=0o700)
    if directory.is_symlink() or directory.stat().st_uid != os.getuid():
        raise ValueError("TLS directory must belong to the development user.")
    directory.chmod(0o700)
    descriptor = os.open(
        directory / ".setup.lock", os.O_CREAT | os.O_RDWR | os.O_NOFOLLOW, 0o600
    )
    with os.fdopen(descriptor, "a") as lock:
        fcntl.flock(lock, fcntl.LOCK_EX)
        provision_material(project, directory)


def ensure_ca(directory):
    ca_key = directory / "ca.key"
    ca_certificate = directory / "ca.crt"
    if ca_key.exists() != ca_certificate.exists():
        raise ValueError("Incomplete CA material; restore it before continuing.")
    if ca_certificate.exists():
        validate_pair(ca_certificate, ca_key)
        openssl("x509", "-in", ca_certificate, "-checkend", "2592000", "-noout")
        openssl("verify", "-CAfile", ca_certificate, ca_certificate)
    else:
        with tempfile.TemporaryDirectory(dir=directory) as temporary:
            staging = Path(temporary)
            openssl(
                "req", "-x509", "-newkey", "rsa:3072", "-sha256", "-nodes",
                "-days", "365", "-keyout", staging / "ca.key",
                "-out", staging / "ca.crt",
                "-subj", "/CN=Sonimbus Gateway Development CA",
                "-addext", "basicConstraints=critical,CA:TRUE,pathlen:0",
                "-addext", "keyUsage=critical,keyCertSign,cRLSign",
            )
            for name in ("ca.key", "ca.crt"):
                (staging / name).replace(directory / name)

    return ca_certificate, ca_key


def ensure_server_certificate(directory, ca_certificate, ca_key):
    certificate, key = directory / "gateway.crt", directory / "gateway.key"
    if certificate.exists() != key.exists():
        raise ValueError("Incomplete gateway material; restore it before continuing.")
    renew = not certificate.exists()
    if not renew:
        validate_pair(certificate, key)
        try:
            openssl("x509", "-in", certificate, "-checkend", "86400", "-noout")
            openssl(
                "verify", "-CAfile", ca_certificate,
                "-verify_hostname", "localhost", certificate,
            )
            openssl(
                "verify", "-CAfile", ca_certificate,
                "-verify_ip", "127.0.0.1", certificate,
            )
        except subprocess.CalledProcessError:
            renew = True
    if renew:
        with tempfile.TemporaryDirectory(dir=directory) as temporary:
            staging = Path(temporary)
            openssl(
                "req", "-new", "-newkey", "rsa:2048", "-nodes",
                "-keyout", staging / "gateway.key", "-out", staging / "gateway.csr",
                "-subj", "/CN=localhost",
            )
            extensions = staging / "server.ext"
            extensions.write_text(
                "subjectAltName=DNS:localhost,IP:127.0.0.1\n"
                "basicConstraints=critical,CA:FALSE\n"
                "keyUsage=critical,digitalSignature,keyEncipherment\n"
                "extendedKeyUsage=serverAuth\n",
                encoding="utf-8",
            )
            openssl(
                "x509", "-req", "-in", staging / "gateway.csr",
                "-CA", ca_certificate, "-CAkey", ca_key,
                "-set_serial", "0x" + os.urandom(16).hex(), "-days", "30", "-sha256",
                "-extfile", extensions, "-out", staging / "gateway.crt",
            )
            for name in ("gateway.key", "gateway.crt"):
                (staging / name).replace(directory / name)
    return certificate, key


def provision_material(project, directory):
    ca_certificate, ca_key = ensure_ca(directory)
    certificate, key = ensure_server_certificate(directory, ca_certificate, ca_key)
    for path in (ca_key, ca_certificate, certificate, key):
        path.chmod(0o600)
    export = project / "secrets/gateway-dev"
    export.mkdir(parents=True, exist_ok=True)
    (export / "ca.crt").write_bytes(ca_certificate.read_bytes())
    print(f"Gateway TLS ready: {directory}")
    print(f"Public development CA: {export / 'ca.crt'}")


def main():
    if os.name != "posix" or os.geteuid() == 0:
        raise ValueError("Run as the regular development VM user.")
    os.umask(0o077)
    project = Path(__file__).resolve().parents[3]
    directory = Path.home() / ".local/share/sonimbus-gateway-dev"
    provision(project, directory)


if __name__ == "__main__":
    try:
        main()
    except (ValueError, OSError, subprocess.CalledProcessError):
        print(
            "Gateway TLS setup failed; check OpenSSL, existing material and permissions.",
            file=sys.stderr,
        )
        sys.exit(1)
