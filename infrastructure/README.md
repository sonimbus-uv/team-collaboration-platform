# Infraestructura

**Responsable principal:** `@tuzc0`

## Responsabilidad

Entorno de desarrollo reproducible y despliegue de los servicios de apoyo:

- Máquina virtual: [`Vagrantfile`](../Vagrantfile) y [`scripts/provision.sh`](../scripts/provision.sh).
- Servicios en [`compose.yaml`](compose.yaml): bases de datos, Redis, broker, almacenamiento, gateway y LiveKit.
- Variables de referencia en [`.env.example`](.env.example). El archivo `.env` real no se versiona.
- Configuración del servidor de voz en [`livekit/`](livekit/).

## Fuera de alcance

- Código de los servicios y de los clientes: cada módulo construye su propia imagen o artefacto; aquí solo se integra en Compose.

## Referencias

- [ADR-006: Vagrant y Docker Compose](../docs/decisions/ADR-006-vagrant-docker-compose.md)
- Instrucciones de uso en el [README](../README.md) del repositorio.
