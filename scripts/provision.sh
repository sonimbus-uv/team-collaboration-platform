#!/usr/bin/env bash

set -Eeuo pipefail

export DEBIAN_FRONTEND=noninteractive

readonly PROJECT_DIR="/vagrant"
readonly INFRA_DIR="${PROJECT_DIR}/infrastructure"
readonly COMPOSE_FILE="${INFRA_DIR}/compose.yaml"
readonly ENV_FILE="${INFRA_DIR}/.env"
readonly ENV_EXAMPLE_FILE="${INFRA_DIR}/.env.example"
readonly VM_USER="vagrant"

log() {
  printf '\n[%s] %s\n' "$(date '+%H:%M:%S')" "$1"
}

fail() {
  printf '\nERROR: %s\n' "$1" >&2
  exit 1
}

trap 'printf "\nERROR: el provisionamiento falló en la línea %s.\n" "$LINENO" >&2' ERR

if [[ "${EUID}" -ne 0 ]]; then
  fail "Este script debe ejecutarse con privilegios de root."
fi

log "Actualizando paquetes e instalando herramientas base"
apt-get update -y
apt-get install -y --no-install-recommends \
  ca-certificates \
  curl \
  git \
  gnupg \
  jq \
  make \
  net-tools \
  unzip

if ! command -v docker >/dev/null 2>&1 || \
   ! docker compose version >/dev/null 2>&1; then
  log "Eliminando paquetes que pueden entrar en conflicto con Docker CE"
  for package in \
    docker.io \
    docker-compose \
    docker-compose-v2 \
    docker-doc \
    docker-buildx \
    podman-docker \
    containerd \
    runc; do
    apt-get remove -y "${package}" >/dev/null 2>&1 || true
  done

  log "Configurando el repositorio oficial de Docker"
  install -m 0755 -d /etc/apt/keyrings
  curl -fsSL https://download.docker.com/linux/ubuntu/gpg \
    -o /etc/apt/keyrings/docker.asc
  chmod a+r /etc/apt/keyrings/docker.asc

  . /etc/os-release
  readonly UBUNTU_SUITE="${UBUNTU_CODENAME:-${VERSION_CODENAME}}"
  readonly SYSTEM_ARCH="$(dpkg --print-architecture)"

  printf '%s\n' \
    'Types: deb' \
    'URIs: https://download.docker.com/linux/ubuntu' \
    "Suites: ${UBUNTU_SUITE}" \
    'Components: stable' \
    "Architectures: ${SYSTEM_ARCH}" \
    'Signed-By: /etc/apt/keyrings/docker.asc' \
    > /etc/apt/sources.list.d/docker.sources

  apt-get update -y

  log "Instalando Docker Engine y Docker Compose"
  apt-get install -y \
    docker-ce \
    docker-ce-cli \
    containerd.io \
    docker-buildx-plugin \
    docker-compose-plugin
else
  log "Docker ya está instalado; se conserva la instalación actual"
fi

log "Habilitando Docker y configurando el usuario vagrant"
systemctl enable --now docker

if id "${VM_USER}" >/dev/null 2>&1; then
  usermod -aG docker "${VM_USER}"
  # La box cloud-image (libvirt) crea el usuario con /bin/sh; bento ya usa bash.
  usermod -s /bin/bash "${VM_USER}"
fi

docker --version
docker compose version

[[ -f "${COMPOSE_FILE}" ]] || fail "No se encontró ${COMPOSE_FILE}."

if [[ ! -f "${ENV_FILE}" ]]; then
  [[ -f "${ENV_EXAMPLE_FILE}" ]] || \
    fail "No existe ${ENV_FILE} ni ${ENV_EXAMPLE_FILE}."

  log "Creando infrastructure/.env a partir de .env.example"
  cp "${ENV_EXAMPLE_FILE}" "${ENV_FILE}"
  # Con virtiofs (libvirt) el dueño se refleja en el anfitrión; sin esto quedaría de root.
  chown "${VM_USER}:${VM_USER}" "${ENV_FILE}"
else
  log "Se conservará el archivo infrastructure/.env existente"
fi

log "Validando Docker Compose"
docker compose \
  --env-file "${ENV_FILE}" \
  -f "${COMPOSE_FILE}" \
  config --quiet

log "Iniciando PostgreSQL, Redis y Mailpit"
docker compose \
  --env-file "${ENV_FILE}" \
  -f "${COMPOSE_FILE}" \
  up -d

log "Estado de los servicios predeterminados"
docker compose \
  --env-file "${ENV_FILE}" \
  -f "${COMPOSE_FILE}" \
  ps

printf '\nProvisionamiento completado.\n'
printf 'Servicios base: PostgreSQL, Redis y Mailpit.\n'
printf 'Ejecuta primero: cd /vagrant/infrastructure\n'
printf 'Cassandra: docker compose --profile messaging up -d\n'
printf 'MinIO:     docker compose --profile storage up -d\n'
printf 'Todos:     docker compose --profile messaging --profile storage up -d\n'