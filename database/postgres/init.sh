#!/bin/sh
# Aplica los scripts de database/postgres/init/ en orden contra la instancia.
#
# Lo ejecuta el servicio postgres-init de infrastructure/compose.yaml en cada
# `docker compose up`. Los scripts son idempotentes.
#
# Variables de entorno:
#   PGHOST, PGPORT, PGUSER, PGPASSWORD, PGDATABASE   conexión del superusuario
#   AUTH_DB_PASSWORD, CORE_DB_PASSWORD,
#   FILES_DB_PASSWORD, REPORTS_DB_PASSWORD           contraseñas por servicio

set -eu

script_dir="$(cd "$(dirname "$0")" && pwd)"

for var in AUTH_DB_PASSWORD CORE_DB_PASSWORD FILES_DB_PASSWORD REPORTS_DB_PASSWORD; do
  eval "value=\${${var}:-}"
  if [ -z "${value}" ]; then
    echo "ERROR: falta la variable ${var} en infrastructure/.env" >&2
    exit 1
  fi
done

for file in "${script_dir}"/init/*.sql; do
  echo "Aplicando $(basename "${file}")"
  PGOPTIONS="-c client_min_messages=warning" psql --no-psqlrc --quiet --set ON_ERROR_STOP=1 \
    -v auth_password="${AUTH_DB_PASSWORD}" \
    -v core_password="${CORE_DB_PASSWORD}" \
    -v files_password="${FILES_DB_PASSWORD}" \
    -v reports_password="${REPORTS_DB_PASSWORD}" \
    -f "${file}"
done

echo "Esquemas por servicio listos."
