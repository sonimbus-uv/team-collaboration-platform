# Infraestructura

**Responsable principal:** `@tuzc0`

## Responsabilidad

Entorno de desarrollo reproducible y despliegue de los servicios de apoyo:

- Máquina virtual: [`Vagrantfile`](../Vagrantfile) y [`scripts/provision.sh`](../scripts/provision.sh).
- Servicios en [`compose.yaml`](compose.yaml): PostgreSQL, ScyllaDB (D-05), Redis, RabbitMQ (D-04), MinIO, Mailpit, gateway y LiveKit.
- Variables de referencia en [`.env.example`](.env.example). El archivo `.env` real no se versiona.
- Configuración del servidor de voz en [`livekit/`](livekit/).

## Estado

`compose.yaml` levanta hoy PostgreSQL, Redis, Mailpit, Cassandra (perfil `messaging`) y MinIO (perfil `storage`). Lo pendiente está en [`docs/PLAN.md`](../docs/PLAN.md#tareas-abiertas): cambiar Cassandra por ScyllaDB (T-01), alinear los `.env.example` (T-02), agregar RabbitMQ (T-03), esquemas por servicio (T-04), *health checks* (T-05), Nginx y LiveKit.

## Fuera de alcance

- Código de los servicios y de los clientes: cada módulo construye su propia imagen o artefacto; aquí solo se integra en Compose.

## Referencias

- [ADR-006: Vagrant y Docker Compose](../docs/decisions/ADR-006-vagrant-docker-compose.md)
- [Registro de decisiones](../docs/decisions/README.md) (D-04, D-05, D-09, D-13, D-14)
- [Despliegue de la expo](../docs/deployment/README.md)
- Instrucciones de uso en el [README](../README.md) del repositorio.
