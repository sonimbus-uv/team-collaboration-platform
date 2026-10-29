# Infraestructura

**Responsable principal:** `@Maple-M136279841` (desde el 2026-10-21; antes `@tuzc0`)

## Responsabilidad

Entorno de desarrollo reproducible y despliegue de los servicios de apoyo:

- Máquina virtual: [`Vagrantfile`](../Vagrantfile) y [`scripts/provision.sh`](../scripts/provision.sh).
- Servicios en [`compose.yaml`](compose.yaml): PostgreSQL, ScyllaDB (D-05), Redis, RabbitMQ (D-04), MinIO, Mailpit, gateway y LiveKit.
- Variables de referencia en [`.env.example`](.env.example). El archivo `.env` real no se versiona.
- Configuración del servidor de voz en [`livekit/`](livekit/).

## Estado

`compose.yaml` levanta hoy:

| Servicio | Inicio | Notas |
|---|---|---|
| `postgres` | Predeterminado | PostgreSQL 17 |
| `postgres-init` | Predeterminado | Se ejecuta y termina: esquemas y usuarios por servicio ([`database/postgres/`](../database/postgres/), T-04) |
| `redis` | Predeterminado | Con contraseña y AOF |
| `mailpit` | Predeterminado | Con *health check* |
| `nginx` | Predeterminado | Gateway de Auth (`@tuzc0`) |
| `scylladb` | Perfil `messaging` | 1 *shard*, `SCYLLA_MEMORY` (750 MB por defecto) y `SCYLLA_RESERVE_MEMORY` (250 MB) dentro de `SCYLLA_MEM_LIMIT` (1200 MB), modo desarrollo (T-01) |
| `minio` | Perfil `storage` | Con *health check* |
| `minio-init` | Perfil `storage` | Se ejecuta y termina: crea `MINIO_BUCKET` (T-05) |

La VM además fija `fs.aio-max-nr` para ScyllaDB e instala Erlang/OTP y Elixir con `mise` según [`.tool-versions`](../.tool-versions).

`vagrant provision` agrega al `.env` existente las variables nuevas de `.env.example` sin cambiar las que ya tiene; revisa los valores `change_this_*` que agregue.

### Al pasar de Cassandra a ScyllaDB

El volumen anterior ya no se usa. Para liberar el espacio dentro de la VM:

```bash
docker volume rm team-collaboration-platform_cassandra_data
```

Pendiente ([`docs/PLAN.md`](../docs/PLAN.md#tareas-abiertas)): RabbitMQ (T-03), *keyspace* de ScyllaDB creado al iniciar, medición de memoria (T-09), Nginx completo y LiveKit.

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

El motor usa un núcleo, `SCYLLA_MEMORY=750M` y `SCYLLA_RESERVE_MEMORY=250M`; el contenedor tiene `SCYLLA_MEM_LIMIT=1200m`. Debe cumplirse `SCYLLA_MEMORY + SCYLLA_RESERVE_MEMORY + ~120 MB <= SCYLLA_MEM_LIMIT`. Sin `--reserve-memory`, ScyllaDB aparta 1.5 GiB para el sistema y no arranca: la configuración de `main` del 2026-10-29 (`2026.1.15`, `--memory 1G`, límite 1536M) falla con `insufficient physical memory: needed 1021313024 available 500000000`. Los esquemas CQL viven en [`database/scylla/`](../database/scylla/); su inicialización automática sigue pendiente.

El volumen `scylla_data` empieza vacío. El volumen anterior `cassandra_data` no se elimina ni se migra. Si Cassandra ya estaba en uso, detener su contenedor con `docker stop <contenedor-cassandra>` antes de iniciar ScyllaDB para liberar el puerto 9042. La conservación de datos requiere una migración separada mediante exportación e importación CQL; no montar el volumen de Cassandra en ScyllaDB.
