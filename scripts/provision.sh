#!/usr/bin/env bash

set -Eeuo pipefail

export DEBIAN_FRONTEND=noninteractive

readonly PROJECT_DIR="/vagrant"
readonly INFRA_DIR="${PROJECT_DIR}/infrastructure"
readonly COMPOSE_FILE="${INFRA_DIR}/compose.yaml"
readonly ENV_FILE="${INFRA_DIR}/.env"
readonly ENV_EXAMPLE_FILE="${INFRA_DIR}/.env.example"
readonly VM_USER="vagrant"
readonly TOOL_VERSIONS_FILE="${PROJECT_DIR}/.tool-versions"

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
  build-essential \
  ca-certificates \
  curl \
  git \
  gnupg \
  jq \
  make \
  net-tools \
  pkg-config \
  nftables \
  unzip \
  inotify-tools \
  postgresql-client
# inotify-tools: recarga automática de Phoenix en desarrollo (Core y Files).
# postgresql-client: psql para revisar los esquemas por servicio (T-04).

# ScyllaDB usa Linux AIO; el límite por defecto (65536) se agota con facilidad.
log "Ajustando fs.aio-max-nr para ScyllaDB"
printf 'fs.aio-max-nr = 1048576\n' > /etc/sysctl.d/60-scylladb.conf
sysctl --load /etc/sysctl.d/60-scylladb.conf

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

log "Instalando la protección de acceso a Auth"
install -D -m 0644 \
  "${PROJECT_DIR}/infrastructure/firewall/auth.nft" \
  /etc/team-collaboration-platform/auth.nft
install -D -m 0644 \
  "${PROJECT_DIR}/infrastructure/firewall/auth-firewall.service" \
  /etc/systemd/system/auth-firewall.service
install -d -m 0755 /etc/systemd/system/docker.service.d
cat > /etc/systemd/system/docker.service.d/auth-firewall.conf <<'EOF'
[Unit]
Requires=auth-firewall.service
After=auth-firewall.service
EOF
systemctl daemon-reload
systemctl enable --now auth-firewall.service
# oneshot units already active must be reloaded after reinstalling their rules.
systemctl reload auth-firewall.service

log "Habilitando Docker y configurando el usuario vagrant"
systemctl enable --now docker

if id "${VM_USER}" >/dev/null 2>&1; then
  usermod -aG docker "${VM_USER}"
  # La box cloud-image (libvirt) crea el usuario con /bin/sh; bento ya usa bash.
  usermod -s /bin/bash "${VM_USER}"
fi

docker --version
docker compose version

if id "${VM_USER}" >/dev/null 2>&1; then
  readonly RUSTUP_BIN="/home/${VM_USER}/.cargo/bin/rustup"

  if [[ ! -x "${RUSTUP_BIN}" ]]; then
    log "Instalando Rust estable con rustup para el usuario ${VM_USER}"
    sudo -u "${VM_USER}" -H bash -c \
      'curl --proto "=https" --tlsv1.2 -sSf https://sh.rustup.rs |
         sh -s -- -y --profile minimal --default-toolchain stable \
           --component rustfmt --component clippy'
  else
    log "Rust ya está instalado; se conserva la instalación actual"
  fi

  sudo -u "${VM_USER}" -H "${RUSTUP_BIN}" --version
  sudo -u "${VM_USER}" -H "/home/${VM_USER}/.cargo/bin/cargo" --version
fi

if id "${VM_USER}" >/dev/null 2>&1; then
  # Erlang/OTP y Elixir con las versiones fijadas en .tool-versions (Core y
  # Files). mise usa binarios precompilados de Erlang para Ubuntu 24.04.
  if ! command -v mise >/dev/null 2>&1; then
    log "Instalando mise"
    apt-get install -y --no-install-recommends extrepo
    extrepo enable mise
    apt-get update -y
    apt-get install -y mise
  else
    log "mise ya está instalado; se conserva la instalación actual"
  fi

  [[ -f "${TOOL_VERSIONS_FILE}" ]] || fail "No se encontró ${TOOL_VERSIONS_FILE}."

  log "Instalando Erlang/OTP y Elixir según .tool-versions"
  sudo -u "${VM_USER}" -H bash -c "
    set -Eeuo pipefail
    cd '${PROJECT_DIR}'
    mise trust '${TOOL_VERSIONS_FILE}'
    mise install --yes
    mise exec -- mix local.hex --force --if-missing
    mise exec -- mix local.rebar --force --if-missing
    mise exec -- elixir --version
  "

  readonly BASHRC="/home/${VM_USER}/.bashrc"
  touch "${BASHRC}"

  # Shims al inicio de .bashrc: el .bashrc de Ubuntu termina antes en shells no
  # interactivas, así que una línea al final no aplicaría a `vagrant ssh -c`.
  # Con el archivo vacío `sed 1i` no inserta nada; en ese caso se escribe.
  if ! grep -q 'mise/shims' "${BASHRC}"; then
    if [[ -s "${BASHRC}" ]]; then
      sed -i '1i # Shims de mise (.tool-versions) también en shells no interactivas\nexport PATH="$HOME/.local/share/mise/shims:$PATH"' \
        "${BASHRC}"
    else
      printf '%s\n' \
        '# Shims de mise (.tool-versions) también en shells no interactivas' \
        'export PATH="$HOME/.local/share/mise/shims:$PATH"' \
        > "${BASHRC}"
    fi
  fi

  # Activación completa solo en shells interactivas.
  if ! grep -q 'mise activate bash' "${BASHRC}"; then
    printf '\n# Versiones de .tool-versions (Erlang, Elixir)\neval "$(mise activate bash)"\n' \
      >> "${BASHRC}"
  fi
  chown "${VM_USER}:${VM_USER}" "${BASHRC}"
fi

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

  # Agrega las variables nuevas de .env.example sin tocar las existentes.
  added=0
  while IFS= read -r line; do
    [[ "${line}" =~ ^([A-Z][A-Z0-9_]*)= ]] || continue
    key="${BASH_REMATCH[1]}"
    if ! grep -q "^${key}=" "${ENV_FILE}"; then
      [[ "${added}" -eq 0 ]] && printf '\n# Agregadas por provision.sh desde .env.example\n' >> "${ENV_FILE}"
      printf '%s\n' "${line}" >> "${ENV_FILE}"
      log "Variable agregada a infrastructure/.env: ${key} (revisa su valor)"
      added=$((added + 1))
    fi
  done < "${ENV_EXAMPLE_FILE}"
fi

log "Validando Docker Compose"
docker compose \
  --env-file "${ENV_FILE}" \
  -f "${COMPOSE_FILE}" \
  config --quiet

log "Iniciando PostgreSQL (con esquemas por servicio), Redis, Mailpit y el gateway"
docker compose \
  --env-file "${ENV_FILE}" \
  -f "${COMPOSE_FILE}" \
  up -d

log "Comprobando los esquemas por servicio de PostgreSQL"
# `docker wait` imprime solo el código de salida y también funciona si el
# contenedor ya terminó (postgres-init tarda menos de un segundo).
init_id="$(
  docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" ps -aq postgres-init
)"
[[ -n "${init_id}" ]] || fail "No se encontró el contenedor postgres-init."
init_exit="$(docker wait "${init_id}")" || init_exit="desconocido"
if [[ "${init_exit}" != "0" ]]; then
  docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" logs postgres-init >&2 || true
  fail "postgres-init terminó con código ${init_exit}."
fi

log "Comprobando la IP del gateway"
nginx_id="$(
  docker compose --env-file "${ENV_FILE}" -f "${COMPOSE_FILE}" ps -q nginx
)"
[[ -n "${nginx_id}" ]] || fail "No se encontró el contenedor Nginx."
nginx_ip="$(
  docker inspect \
    --format '{{(index .NetworkSettings.Networks "team-collaboration-platform_auth_gateway").IPAddress}}' \
    "${nginx_id}"
)"
[[ "${nginx_ip}" == "172.30.250.2" ]] || \
  fail "La IP de Nginx no coincide con la IP autorizada por Auth y el firewall."

log "Estado de los servicios predeterminados"
docker compose \
  --env-file "${ENV_FILE}" \
  -f "${COMPOSE_FILE}" \
  ps

printf '\nProvisionamiento completado.\n'
printf 'Servicios base: PostgreSQL, Redis y Mailpit.\n'
printf 'Ejecuta primero: cd /vagrant/infrastructure\n'
printf 'ScyllaDB:  docker compose --profile messaging up -d\n'
printf 'MinIO:     docker compose --profile storage up -d\n'
printf 'Todos:     docker compose --profile messaging --profile storage up -d\n'