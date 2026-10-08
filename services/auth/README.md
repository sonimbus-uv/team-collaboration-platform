# Auth Service

**Responsable principal:** `@tuzc0`

## Responsabilidad

El Auth Service administra la identidad y el acceso de los usuarios del sistema.

Incluye:

- Registro de cuentas (CU-01).
- Verificación de correo (CU-02).
- Inicio de sesión con correo y contraseña (CU-03) y con Google mediante OAuth 2.0 / OpenID Connect (CU-04).
- Recuperación de contraseña (CU-05).
- Cierre de sesión (CU-06).
- Gestión de sesiones: una activa por tipo de cliente (D-12) y límite de intentos fallidos.
- Emisión y renovación de tokens firmados con EdDSA (D-07) y publicación de la clave pública para que los demás servicios los verifiquen.
- Perfil de usuario (CU-07): nombre visible y foto elegida de un paquete predefinido (D-06). Cada cambio publica el evento `user.updated`.

Este servicio es el dueño de los datos de identidad y perfil del usuario. Otros dominios como grupos, proyectos, canales, tareas, mensajería y archivos pertenecen a otros servicios.

Ver la trazabilidad completa en [`docs/requirements/`](../../docs/requirements/README.md#auth-service).

## Stack

- Rust.
- Axum.
- Tokio.
- Tower.
- SQLx.
- PostgreSQL, esquema `auth` (D-13).
- Argon2id para contraseñas.
- EdDSA para tokens firmados.
- Redis para sesiones, caché o rate limiting cuando corresponda.

## Estado Actual

El servicio ya fue inicializado como proyecto Rust/Axum.

Actualmente cuenta con:

- Configuración mediante variables de entorno.
- Conexión a PostgreSQL usando SQLx.
- Ejecución de migraciones SQLx al iniciar el servicio.
- Migración inicial del schema `auth`.
- Endpoint `GET /health`.
- Endpoint `GET /health/db` para verificar conexión con PostgreSQL.
- Manejo básico de errores HTTP con formato Problem Details.
- Registro de cuentas con contraseña Argon2id y token de verificación atómicos.
- Envío de verificación mediante Resend, confirmación y reenvío con cooldown.
- Hash fuera del runtime asíncrono y límites de concurrencia y solicitudes.

Ver [registro, configuración y pruebas](REGISTRATION.md) para el flujo actualizado.

Pendiente:

- Login.
- Refresh tokens.
- Logout.
- Recuperación de contraseña.
- Integración con Redis.
- Emisión de tokens firmados y *endpoint* de la clave pública.
- Inicio de sesión con Google.
- Perfil (nombre visible y foto predefinida) y evento `user.updated`.
- Límite de intentos fallidos.
- Usuario de base de datos propio con permisos solo sobre el esquema `auth` (D-13, tarea T-04 en [`docs/PLAN.md`](../../docs/PLAN.md)).
- Limpiar los archivos vacíos `src/routes/health.rs` y `src/handlers/` (T-06).

## Estructura

```text
services/auth/
├── Cargo.toml
├── Cargo.lock
├── .env.example
├── migrations/
│   └── 0001_initial_auth_schema.sql
└── src/
    ├── main.rs
    ├── config.rs
    ├── db.rs
    ├── error.rs
    ├── state.rs
    └── routes/
        └── mod.rs
```

## Variables de Entorno

Crear un archivo `.env` local a partir de `.env.example`:

```bash
cp services/auth/.env.example services/auth/.env
```

Ejemplo:

```env
SERVER_ADDR=0.0.0.0:3001
DATABASE_URL=postgres://team_user:change_me_postgres@localhost:5432/team_collaboration
DATABASE_MAX_CONNECTIONS=10
```

Si el servicio se ejecuta fuera de Docker, `DATABASE_URL` debe apuntar a `localhost` cuando PostgreSQL esté expuesto en el host:

```env
DATABASE_URL=postgres://team_user:change_me_postgres@localhost:5432/team_collaboration
```

Si el servicio se ejecuta dentro del mismo Docker Compose que PostgreSQL, `DATABASE_URL` debe usar el nombre del servicio:

```env
DATABASE_URL=postgres://team_user:change_me_postgres@postgres:5432/team_collaboration
```

El archivo `.env` no debe subirse al repositorio.

## Base de Datos

Las migraciones viven dentro del servicio:

```text
services/auth/migrations/
```

La migración inicial crea el schema `auth` y las tablas principales del servicio.

Para levantar PostgreSQL desde el archivo Compose del proyecto:

```bash
docker compose -f infrastructure/compose.yaml up -d postgres
```

Para verificar el estado del contenedor:

```bash
docker compose -f infrastructure/compose.yaml ps
```

## Ejecutar el Servicio

Desde la raíz del proyecto:

```bash
docker compose -f infrastructure/compose.yaml up -d postgres
```

Entrar al directorio del servicio:

```bash
cd services/auth
```

Verificar compilación:

```bash
cargo check
```

Ejecutar:

```bash
cargo run
```

El servicio ejecuta las migraciones SQLx al iniciar.

## Verificar Funcionamiento

Los ejemplos asumen `SERVER_ADDR=0.0.0.0:3001`.

Health del servicio:

```bash
curl http://localhost:3001/health
```

Respuesta esperada:

```json
{
  "status": "ok"
}
```

Health de base de datos:

```bash
curl http://localhost:3001/health/db
```

Respuesta esperada:

```json
{
  "status": "ok"
}
```

Si PostgreSQL no está disponible, `GET /health/db` debe responder con `503 Service Unavailable` usando formato Problem Details.

Ejemplo:

```json
{
  "type": "about:blank",
  "title": "Database unavailable",
  "status": 503,
  "detail": "The service cannot connect to the database."
}
```

## Comandos de Desarrollo

Formatear código:

```bash
cargo fmt
```

Verificar compilación:

```bash
cargo check
```

Ejecutar:

```bash
cargo run
```

Si se trabaja dentro de una carpeta compartida de Vagrant y aparecen errores relacionados con `target/`, se puede usar un directorio de compilación fuera de la carpeta compartida:

```bash
CARGO_TARGET_DIR=~/.cargo-target/auth-service cargo check
```

```bash
CARGO_TARGET_DIR=~/.cargo-target/auth-service cargo run
```

## Contratos

Expone:

```text
contracts/openapi/auth.yaml
```

Los demás servicios validarán los tokens usando la clave pública correspondiente. No deben llamar a Auth para validar cada petición.

## Fuera de Alcance

No pertenecen a este servicio:

- Grupos.
- Roles dentro de grupos.
- Proyectos.
- Tareas.
- Canales.
- Mensajería.
- Archivos.
- Archivos: la foto de perfil se elige de un paquete predefinido incluido en los clientes y Auth solo guarda su identificador (D-06). Cualquier otro archivo se gestiona en el File Service.
