#!/usr/bin/env bash
set -euo pipefail
umask 077

PROJECT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../../.." && pwd)"

if [[ $# -ne 0 ]]; then
  echo "Usage: bash scripts/auth/dev/setup-auth-dev.sh"
  exit 1
fi

if [[ -e "$PROJECT_DIR/services/auth/.env" ]]; then
  echo "Existing Auth configuration preserved. Run scripts/auth/dev/start-auth-dev.sh."
  exit 0
fi

for tool in python3 openssl docker; do
  command -v "$tool" >/dev/null || { echo "Missing tool: $tool" >&2; exit 1; }
done

if [[ $EUID -eq 0 ]]; then
  echo "Run as the development VM user, not root." >&2
  exit 1
fi

python3 "$PROJECT_DIR/scripts/auth/dev/auth-dev-setup.py" "$PROJECT_DIR"
