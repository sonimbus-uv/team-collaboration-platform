"""Exercise the real Nginx files against a disposable public-key HTTP stub.

Requires nginx on PATH (or --nginx). No Auth process, Redis or database needed.
Only generated config copies in a TemporaryDirectory are changed.
"""
import argparse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
import json
from pathlib import Path
import shutil
import socket
import subprocess
import tempfile
import threading
import time
from urllib.error import HTTPError, URLError
from urllib.request import Request, urlopen


CACHE = "public, max-age=300, must-revalidate"
BODY = json.dumps({"keys": [{"kty": "OKP", "crv": "Ed25519", "use": "sig",
                            "alg": "EdDSA", "kid": "test-only", "x": "test-public-stub"}]}).encode()


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


def request(port, path, method="GET"):
    req = Request(f"http://127.0.0.1:{port}{path}", method=method)
    try:
        response = urlopen(req, timeout=2)
    except HTTPError as error:
        response = error
    with response:
        return response.status, response.headers, response.read()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--nginx", default=shutil.which("nginx"))
    args = parser.parse_args()
    if not args.nginx:
        parser.error("nginx is required; install it or supply --nginx")
    root = Path(__file__).resolve().parents[1]
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
                    text = text.replace("host.docker.internal:3001", f"127.0.0.1:{backend.server_port}")
                    text = text.replace("/dev/stdout", "logs/access.log")
                    (directory / subdir / source.name).write_text(text, encoding="utf-8")
            config = (root / "gateway/nginx/nginx.conf").read_text(encoding="utf-8")
            config = config.replace("listen 80;", f"listen 127.0.0.1:{port};")
            config = config.replace("/dev/null", "logs/error.log")
            config = config.replace("/etc/nginx/", directory.as_posix() + "/")
            config = "pid logs/nginx.pid;\n" + config
            paths = "\n".join(f"{kind}_temp_path temp/{kind};" for kind in
                              ["client_body", "proxy", "fastcgi", "uwsgi", "scgi"])
            config = config.replace("http {", "http {\n" + paths)
            (directory / "nginx.conf").write_text(config, encoding="utf-8")
            command = [args.nginx, "-p", directory.as_posix() + "/", "-c", "nginx.conf"]
            subprocess.run(command + ["-t"], check=True)
            process = subprocess.Popen(command + ["-g", "daemon off;"])
            try:
                deadline = time.monotonic() + 10
                while True:
                    try:
                        status, _, _ = request(port, "/health")
                        assert status == 200
                        break
                    except (URLError, OSError):
                        if time.monotonic() >= deadline:
                            raise
                        time.sleep(0.1)
                for _ in range(24):
                    status, headers, body = request(port, "/.well-known/jwks.json")
                    assert status == 200 and body == BODY
                    assert headers["Cache-Control"] == CACHE
                    assert headers["Content-Type"] == "application/json"
                status, headers, body = request(port, "/.well-known/jwks.json", "HEAD")
                assert status == 200 and not body and headers["Cache-Control"] == CACHE
                statuses = [request(port, "/auth/register", "POST")[0] for _ in range(16)]
                assert 429 in statuses, "existing registration edge budget must remain enabled"
                assert request(port, "/.well-known/jwks.json")[0] == 200
                assert request(port, "/not-a-route")[0] == 404
                print("PASS: nginx -t, public JWKS GET/HEAD, cache headers, isolated budgets and default 404")
            finally:
                subprocess.run(command + ["-s", "quit"], check=False, capture_output=True)
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
