# PostgreSQL

Inicialización de la instancia: un esquema y un usuario por servicio, con permisos solo sobre su esquema (D-13, T-04). Las migraciones de cada servicio viven en su propio directorio. Ver [`database/README.md`](../README.md).

## Qué crea

| Esquema | Usuario | Contraseña (`infrastructure/.env`) | `search_path` |
|---|---|---|---|
| `auth` | `auth_service` | `AUTH_DB_PASSWORD` | `auth, public` |
| `core` | `core_service` | `CORE_DB_PASSWORD` | `core, public` |
| `files` | `files_service` | `FILES_DB_PASSWORD` | `files, public` |
| `reports` | `reports_service` | `REPORTS_DB_PASSWORD` | `reports, public` |

Además:

- Retira a `PUBLIC` el acceso a la base de datos y la creación en `public`; cada usuario solo puede conectarse y crear objetos dentro de su esquema.
- Instala la extensión `citext` en `public` (la usa Auth). Los usuarios de servicio no pueden instalar extensiones.
- Con el `search_path` anterior, la tabla de migraciones de cada servicio (`_sqlx_migrations` en Auth, `schema_migrations` en Ecto) queda dentro de su esquema.

**Excepción temporal:** `auth_service` tiene `CREATE` sobre la base de datos porque la migración `0001` de Auth ejecuta `CREATE SCHEMA IF NOT EXISTS auth`, y PostgreSQL comprueba ese permiso antes que la existencia del esquema. Se retira cuando Auth quite esa sentencia.

## Cómo se ejecuta

El servicio `postgres-init` de [`infrastructure/compose.yaml`](../../infrastructure/compose.yaml) corre [`init.sh`](init.sh) en cada `docker compose up`, cuando PostgreSQL está *healthy*, y termina. Los scripts de [`init/`](init/) se aplican en orden y son idempotentes: crean lo que falta y vuelven a sincronizar las contraseñas con el `.env`.

Para revisar el resultado:

```bash
cd /vagrant/infrastructure
docker compose logs postgres-init
psql -h 127.0.0.1 -U core_service -d team_collaboration -c 'show search_path'
```

Un script nuevo se agrega como `init/NNN_descripcion.sql` y debe poder ejecutarse varias veces sin error.

## Volúmenes anteriores a T-04

Si el volumen `postgres_data` ya tenía el esquema `auth` creado por la migración de Auth con el superusuario, `postgres-init` no lo modifica y muestra un aviso: las tablas y el historial de migraciones siguen siendo del superusuario, y Auth no puede usarlos como `auth_service`. Como en desarrollo los datos son desechables, la forma recomendada es recrear el volumen una vez:

```bash
cd /vagrant/infrastructure
docker compose down
docker volume rm team-collaboration-platform_postgres_data
docker compose up -d
```

Después, Auth debe conectarse con `auth_service` (`DATABASE_URL=postgres://auth_service:<AUTH_DB_PASSWORD>@localhost:5432/team_collaboration`) y volver a aplicar sus migraciones.
