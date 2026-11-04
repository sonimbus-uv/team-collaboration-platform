# Automated development environment

These scripts target the provisioned Linux Vagrant VM, not production. They do
not install tools, apply new firewall rules, rotate credentials, or remove data.
Run them as the regular VM user with Docker access and sudo access for systemd.

## First-time setup

From PowerShell in the repository root:

```powershell
vagrant up
vagrant ssh
```

Inside the VM:

```bash
cd /vagrant
bash scripts/auth/dev/setup-auth-dev.sh
```

The VM must already be provisioned, including `infrastructure/.env`, Rust,
Docker Compose, OpenSSL, Python 3 and `auth-firewall.service`. Setup reads the
effective Compose credentials and published dependency ports. It never sources
the environment files as shell code or prints their passwords.

If `services/auth/.env` exists, setup exits successfully without changing it or
its secrets. It does not repair existing configuration. This preserves existing
developer environments, including their certificate and token paths.

For a new environment, setup creates:

- An ignored `services/auth/.env`, with login disabled and the private HTTPS API
  enabled on loopback only.
- A private, unique directory under `~/.local/share/sonimbus-auth-dev-*` containing
  Ed25519 signing material, service credential hashes and original `core.token` /
  `files.token` files. Independent HMAC secrets are stored in the ignored `.env`.
- A development CA valid for one year and a localhost server certificate valid
  for 30 days. Expiration requires deliberate renewal; rerunning setup does not
  rotate existing material.
- Copies of public certificates under ignored `secrets/sonimbus-auth-dev-*` for
  Windows/Postman. Import `dev-ca.crt` as a CA certificate and keep TLS verification
  enabled. Never distribute the CA private key.

The secrets directory is mode 0700 and its files are mode 0600 on Linux. The
shared folder's Windows permissions must also restrict access to the local `.env`.
No secrets are printed, committed, or exported to Postman automatically.

If setup fails, existing configuration is not overwritten. Any generated files
remain in the reported private directory for inspection. Fix the cause and rerun;
do not remove existing working configuration as a troubleshooting shortcut.

## Daily startup

```bash
cd /vagrant
bash scripts/auth/dev/start-auth-dev.sh --enable-login
```

Omit `--enable-login` to keep login disabled, even if `.env` enables it. Explicit
login enablement is only for isolated development while the registration and
legacy-account reviews remain pending. Internal API enablement still comes from
your existing `.env`.

The script starts the installed firewall service, checks its table exists,
starts PostgreSQL, Redis, Mailpit and Nginx, waits up to 120 seconds for Compose
readiness, and runs Cargo in the foreground. Compilation output defaults to
`~/.cargo-target/auth-service`; an existing `CARGO_TARGET_DIR` is respected.
Auth applies migrations to the configured database at startup.

Ctrl+C stops Auth. The containers and firewall remain active. A second Auth
process cannot use ports already held by the first. The firewall service and its
rules must have been installed by the existing VM provisioning; these scripts
do not alter its network policy or apply the internal firewall example.

## Windows access and Postman

In a separate PowerShell terminal in the repository root:

```powershell
vagrant ssh -- -N -L 18080:127.0.0.1:8080 -L 18025:127.0.0.1:8025 -L 18443:127.0.0.1:3443
```

Keep the terminal open. With the standard Compose ports, use:

| URL | Purpose |
|---|---|
| `http://localhost:18080/health` | Gateway / Auth |
| `http://localhost:18025` | Mailpit |
| `https://localhost:18443` | Private API |

If Mailpit's published web port differs, adjust the tunnel destination. Read
service tokens locally from the private directory reported by setup and store
them only in local Postman values. Never paste tokens into chat or commit them.
Existing environments keep their existing certificate trust configuration.

Run the public and private collections separately, one iteration each. The
private collection requires both service tokens and the configured CA trust.

## Troubleshooting and validation

- Docker permissions: grant the VM user the intended Docker access and reconnect;
  do not run the entire setup or Auth as root.
- Existing PostgreSQL volumes retain their original passwords. Changing
  `infrastructure/.env` does not update passwords inside an existing database.
- Missing firewall service or table: review VM provisioning before startup.
- Missing keys or invalid certificate: check the paths in the existing `.env`.
- Container readiness is not a substitute for `/health/db` and `/health/redis`.

Syntax and isolated provisioning checks (no real database or Docker mutation):

```bash
bash -n scripts/auth/dev/setup-auth-dev.sh scripts/auth/dev/start-auth-dev.sh
python3 scripts/auth/tests/test-auth-dev-setup.py
```

The provisioning test uses real OpenSSL and temporary files, with resolved Compose
configuration mocked. It verifies certificate trust, distinct credential hashes,
port/password encoding, Unix permissions and preservation on repeat execution.
