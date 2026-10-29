# Infraestructura

**Responsable principal:** `@tuzc0`

## Responsabilidad

Entorno de desarrollo reproducible y despliegue de los servicios de apoyo:

- Máquina virtual: [`Vagrantfile`](../Vagrantfile) y [`scripts/provision.sh`](../scripts/provision.sh).
- Servicios en [`compose.yaml`](compose.yaml): PostgreSQL, ScyllaDB (D-05), Redis, RabbitMQ (D-04), MinIO, Mailpit, gateway y LiveKit.
- Variables de referencia en [`.env.example`](.env.example). El archivo `.env` real no se versiona.
- Configuración del servidor de voz en [`livekit/`](livekit/).

## Estado

`compose.yaml` levanta hoy PostgreSQL, Redis, Mailpit, ScyllaDB (perfil `messaging`) y MinIO (perfil `storage`). Lo pendiente está en [`docs/PLAN.md`](../docs/PLAN.md#tareas-abiertas): alinear los `.env.example` (T-02), agregar RabbitMQ (T-03), esquemas por servicio (T-04), *health checks* (T-05), Nginx y LiveKit.

## Fuera de alcance

- Código de los servicios y de los clientes: cada módulo construye su propia imagen o artefacto; aquí solo se integra en Compose.

## Referencias

- [ADR-006: Vagrant y Docker Compose](../docs/decisions/ADR-006-vagrant-docker-compose.md)
- [Registro de decisiones](../docs/decisions/README.md) (D-04, D-05, D-09, D-13, D-14)
- [Despliegue de la expo](../docs/deployment/README.md)
- Instrucciones de uso en el [README](../README.md) del repositorio.

## Uso de ScyllaDB

Desde `infrastructure/` (en la VM, `/vagrant/infrastructure`):

```bash
docker compose --profile messaging up -d scylladb --wait
docker compose exec scylladb cqlsh -e 'SELECT release_version FROM system.local'
```

Usar `scylladb:9042` desde contenedores en `backend` y `localhost:9042` desde el host de Docker (puerto configurable con `SCYLLA_PORT`). Desde el equipo anfitrión de Vagrant, usar `192.168.33.30:9042`.

El motor usa un núcleo y `SCYLLA_MEMORY=1G`; el contenedor tiene `SCYLLA_MEM_LIMIT=1536M`. Mantener margen entre ambos valores. Los esquemas CQL viven en [`database/scylla/`](../database/scylla/); su inicialización automática sigue pendiente.

El volumen `scylla_data` empieza vacío. El volumen anterior `cassandra_data` no se elimina ni se migra. Si Cassandra ya estaba en uso, detener su contenedor con `docker stop <contenedor-cassandra>` antes de iniciar ScyllaDB para liberar el puerto 9042. La conservación de datos requiere una migración separada mediante exportación e importación CQL; no montar el volumen de Cassandra en ScyllaDB.
