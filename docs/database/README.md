# Modelo de datos

Modelo conceptual y lógico de los datos de cada servicio. Los scripts y esquemas ejecutables viven en [`database/`](../../database/) y en cada servicio (las migraciones).

## PostgreSQL (D-13)

Una instancia y una base de datos compartidas, con un esquema y un usuario por servicio. Cada usuario solo tiene permisos sobre su esquema.

| Esquema | Usuario | Servicio | Migraciones | Estado |
|---|---|---|---|---|
| `auth` | `auth_service` | Auth | `services/auth/migrations/` (SQLx) | Migración inicial lista; hoy se conecta con el superusuario (T-04) |
| `core` | `core_service` | Core | `services/core/priv/repo/migrations/` (Ecto) | Pendiente |
| `files` | `files_service` | File Service | Dentro del servicio | Pendiente |
| `reports` | `reports_service` | Report Service | Dentro del servicio | Pendiente |

Los nombres de usuario son una propuesta; los define el script de inicialización de [`database/postgres/`](../../database/postgres/) (tarea T-04).

### Esquema `auth`

Implementado en `services/auth/migrations/0001_initial_auth_schema.sql`:

| Tabla | Propósito |
|---|---|
| `accounts` | Identidad: correo (`citext`), verificación y estado administrativo |
| `password_credentials` | Hash de contraseña; existe solo si la cuenta puede entrar con contraseña |
| `oauth_identities` | Identidades de Google vinculadas a una cuenta |
| `sessions` | Una sesión activa por `(cuenta, tipo de cliente)`; se abre con `auth.open_session()` (D-12) |
| `refresh_tokens` | Tokens de renovación con rotación y detección de reutilización |
| `one_time_tokens` | Códigos de verificación, cambio de correo y recuperación de contraseña |
| `email_change_requests` | Solicitudes de cambio de correo pendientes |

Pendiente en Auth: tablas o columnas de perfil (nombre visible e identificador de la foto predefinida, D-06) e intentos fallidos.

## ScyllaDB (D-05)

Mensajes y reacciones de Core. Los esquemas CQL viven en [`database/scylla/`](../../database/scylla/), numerados en orden de aplicación.

| Necesidad | Diseño |
|---|---|
| Historial paginado por canal (CU-20) | Clave de partición `(channel_id, bucket)` con un *bucket* por mes; clave de ordenamiento `message_id` (timeuuid) descendente; cursor por el último `message_id` |
| Idempotencia (RNF-CON-01) | Tabla auxiliar por `client_message_id` con `INSERT ... IF NOT EXISTS` |
| Editar y eliminar (CU-19) | Actualizar por clave; eliminar con un indicador `deleted`, no con `DELETE`, para evitar *tombstones* |
| Reacciones (CU-18) | Una fila por `(message_id, emoji, user_id)`, contando al leer. **No usar** el tipo `counter`, porque no es idempotente |
| Desarrollo | Un nodo, factor de replicación 1, consistencia `ONE`, memoria acotada |
| Driver de Elixir | Xandra |

## Redis

| Uso | Servicio |
|---|---|
| Sesiones e intentos fallidos | Auth |
| Presencia y caché de permisos | Core |

## MinIO

Contenido de los archivos y vistas previas, propiedad del File Service. Los metadatos (nombre, tipo, tamaño, autor, canal y estado) van en el esquema `files` de PostgreSQL.
