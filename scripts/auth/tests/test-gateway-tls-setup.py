"""Verify real certificate provisioning, renewal and private material isolation."""

import importlib.util
import os
from pathlib import Path
import subprocess
import tempfile
import unittest

spec = importlib.util.spec_from_file_location(
    "gateway_tls", Path(__file__).resolve().parents[1] / "dev/setup-gateway-tls.py"
)
setup = importlib.util.module_from_spec(spec)
spec.loader.exec_module(setup)


class GatewayTlsTests(unittest.TestCase):
    def test_reuse_leaf_renewal_and_ca_preservation(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            private = root / "private"
            project = root / "project"
            old_mask = os.umask(0o077)
            try:
                setup.provision(project, private)
                original = {path.name: path.read_bytes() for path in private.iterdir()}
                setup.provision(project, private)
                self.assertEqual(original, {path.name: path.read_bytes() for path in private.iterdir()})
                self.assertEqual({"ca.crt"}, {path.name for path in (project / "secrets/gateway-dev").iterdir()})
                setup.openssl("verify", "-CAfile", private / "ca.crt", "-purpose", "sslserver", "-verify_hostname", "localhost", private / "gateway.crt")
                setup.openssl("verify", "-CAfile", private / "ca.crt", "-verify_ip", "127.0.0.1", private / "gateway.crt")
                with self.assertRaises(subprocess.CalledProcessError):
                    setup.openssl("verify", "-CAfile", private / "ca.crt", "-verify_hostname", "unrelated.invalid", private / "gateway.crt")
                setup.openssl(
                    "req", "-x509", "-new", "-key", private / "gateway.key",
                    "-days", "1", "-subj", "/CN=expired-soon", "-out", private / "gateway.crt",
                )
                setup.provision(project, private)
                self.assertEqual(original["ca.crt"], (private / "ca.crt").read_bytes())
                self.assertEqual(original["ca.key"], (private / "ca.key").read_bytes())
                self.assertNotEqual(original["gateway.crt"], (private / "gateway.crt").read_bytes())
                setup.validate_pair(private / "gateway.crt", private / "gateway.key")
                self.assertEqual(private.stat().st_mode & 0o777, 0o700)
                for path in private.iterdir():
                    self.assertEqual(path.stat().st_mode & 0o777, 0o600)
            finally:
                os.umask(old_mask)

    def test_incomplete_ca_is_not_replaced(self):
        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            private = root / "private"
            private.mkdir()
            key = private / "ca.key"
            key.write_bytes(b"existing-private-material")
            with self.assertRaises(ValueError):
                setup.provision(root / "project", private)
            self.assertEqual(key.read_bytes(), b"existing-private-material")
            self.assertFalse((private / "ca.crt").exists())


if __name__ == "__main__":
    unittest.main()
