#!/usr/bin/env bash
set -euo pipefail

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"
LOGIN_ENABLED=false

case "${1:-}" in
  "") ;;
  --enable-login) LOGIN_ENABLED=true ;;
  *) echo "Usage: bash scripts/auth/dev/start-auth-dev.sh [--enable-login]"; exit 1 ;;
esac
[[ $# -le 1 ]] || { echo "Too many arguments." >&2; exit 1; }

for file in services/auth/.env infrastructure/.env; do
  [[ -f "$PROJECT_DIR/$file" ]] || {
    echo "Missing $file. Provision the VM and run scripts/auth/dev/setup-auth-dev.sh." >&2
    exit 1
  }
done

if [[ -f "$HOME/.cargo/env" ]]; then
  source "$HOME/.cargo/env"
fi
for tool in cargo docker systemctl sudo; do
  command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 1; }
done
if [[ $EUID -eq 0 ]]; then
  echo "Run as the development VM user, not root; private keys belong to that user." >&2
  exit 1
fi

echo "Starting the provisioned Auth firewall..."
sudo systemctl start auth-firewall.service
sudo nft list table inet auth_guard >/dev/null

echo "Starting dependencies and gateway..."
docker compose --env-file "$PROJECT_DIR/infrastructure/.env" \
  -f "$PROJECT_DIR/infrastructure/compose.yaml" \
  up -d --wait --wait-timeout 120 postgres redis mailpit nginx

echo "Starting Auth; development login enabled: $LOGIN_ENABLED"
echo "Ctrl+C stops Auth. Containers and firewall remain running."
cd "$PROJECT_DIR/services/auth"
export AUTH_LOGIN_ENABLED="$LOGIN_ENABLED"
export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$HOME/.cargo-target/auth-service}"
exec cargo run --locked
