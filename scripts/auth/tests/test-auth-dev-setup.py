"""Exercise provisioning with real OpenSSL and isolated Compose configuration."""

import hashlib
import importlib.util
import json
from pathlib import Path
import subprocess
import tempfile
import unittest
from unittest.mock import patch

spec = importlib.util.spec_from_file_location("auth_dev_setup", Path(__file__).resolve().parents[1] / "dev/auth-dev-setup.py")
setup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(setup)


class ProvisioningTests(unittest.TestCase):
    def test_generation_and_existing_configuration_preservation(self):
        with tempfile.TemporaryDirectory() as temporary:
            project = Path(temporary) / "project"
            home = Path(temporary) / "home"
            auth = project / "services/auth"
            auth.mkdir(parents=True)
            (project / "infrastructure").mkdir()
            (project / "infrastructure/.env").write_text("placeholder\n")
            source = Path(__file__).resolve().parents[3] / "services/auth/.env.example"
            (auth / ".env.example").write_text(source.read_text())
            config = {"services": {
                "postgres": {"environment": {"POSTGRES_USER": "test", "POSTGRES_PASSWORD": "p@ss:/word",
                                             "POSTGRES_DB": "auth_test"},
                             "ports": [{"target": 5432, "published": "15432"}]},
                "redis": {"environment": {"REDIS_PASSWORD": "r@ss"},
                          "ports": [{"target": 6379, "published": "16379"}]},
                "mailpit": {"ports": [{"target": 1025, "published": "11025"}]},
            }}
            real_run = subprocess.run

            def run(command, **kwargs):
                if command[0] == "docker":
                    return subprocess.CompletedProcess(command, 0, json.dumps(config), "")
                return real_run(command, **kwargs)

            with patch.object(setup.Path, "home", return_value=home), \
                 patch.object(setup.sys, "argv", ["setup", str(project)]), \
                 patch.object(setup.subprocess, "run", side_effect=run):
                setup.main()
                original = (auth / ".env").read_bytes()
                setup.main()
                self.assertEqual(original, (auth / ".env").read_bytes())

            folder, = (home / ".local/share").iterdir()
            credentials = json.loads((folder / "internal-credentials.json").read_text())
            tokens = []
            for name in ("core", "files"):
                token = (folder / f"{name}.token").read_text().strip()
                tokens.append(token)
                self.assertEqual(len(token), 47)
                self.assertEqual(credentials[name]["active_sha256"], hashlib.sha256(token.encode()).hexdigest())
            self.assertNotEqual(*tokens)
            env = (auth / ".env").read_text()
            self.assertIn("p%40ss%3A%2Fword@127.0.0.1:15432", env)
            self.assertIn('AUTH_LOGIN_ENABLED="false"', env)
            self.assertIn('MAILPIT_SMTP_PORT="11025"', env)
            real_run(["openssl", "verify", "-CAfile", str(folder / "dev-ca.crt"),
                      "-verify_hostname", "localhost", str(folder / "internal-tls.crt")],
                     check=True, capture_output=True)
            exported = project / "secrets" / folder.name
            self.assertEqual({p.name for p in exported.iterdir()}, {"dev-ca.crt", "internal-tls.crt"})
            if setup.os.name == "posix":
                self.assertEqual(folder.stat().st_mode & 0o777, 0o700)
                for path in folder.iterdir():
                    self.assertEqual(path.stat().st_mode & 0o777, 0o600)


if __name__ == "__main__":
    unittest.main()
