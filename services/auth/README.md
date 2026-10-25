# Auth Service

**Responsable principal:** `@tuzc0`

## Responsabilidad

El Auth Service administra la identidad y el acceso de los usuarios del sistema.

El alcance funcional previsto incluye (no todo está implementado; ver Estado Actual):

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
- Redis para rate limiting; las sesiones y los refresh se persisten en PostgreSQL.

## Estado Actual

El servicio ya fue inicializado como proyecto Rust/Axum.

Actualmente cuenta con:

- Configuración mediante variables de entorno.
- Conexión a PostgreSQL usando SQLx.
- Ejecución de migraciones SQLx al iniciar el servicio.
- Migración inicial del schema `auth`.
- Endpoint `GET /health`.
- Endpoint `GET /health/db` para verificar conexión con PostgreSQL.
- Endpoint `GET /health/redis` para comprobar Redis con PING.
- Manejo básico de errores HTTP con formato Problem Details.
- Registro de cuentas con contraseña Argon2id y token de verificación atómicos.
- Envío de verificación mediante Resend o Mailpit, confirmación y reenvío con cooldown.
- Verificación desde el enlace del correo mediante GET con respuesta HTML, conservando POST con JSON para clientes móviles y de escritorio.
- Hash fuera del runtime asíncrono y límites de concurrencia y solicitudes.
- Rate limiting distribuido con Redis por operación, IP y destinatario.
- Configuración de Nginx y firewall persistente para proteger la entrada; los
  registros del gateway omiten credenciales y query. Ver limitaciones de logs de
  Auth en [RATE_LIMITING.md](RATE_LIMITING.md).
- Componente de emisión JWT Ed25519 con contexto de sesión validado y claves cargadas desde archivos.
- Endpoint público `GET/HEAD /.well-known/jwks.json`, con caché de cinco minutos y claves retenidas para rotación.
- Sesiones transaccionales, rotación estricta de refresh y logout con intención de revocación en outbox.
- Rutas `/api/v1/auth/login`, `/api/v1/auth/refresh`, `/api/v1/auth/logout`; login deshabilitado por defecto por #17.

Ver [registro, configuración y pruebas](REGISTRATION.md) para el flujo actualizado.

Ver [rate limiting, configuración y diagnóstico](RATE_LIMITING.md).

Ver [firma JWT, configuración obligatoria de claves, JWKS y rotación](TOKENS.md).
Auth requiere archivos de claves aprovisionados explícitamente para arrancar;
copiar `.env.example` por sí solo no es suficiente. No hay generación automática.

Ver [sesiones, bloqueo de habilitación por #17, transacciones y pruebas](SESSIONS.md).

Ver [revisión documental y evidencia de validación](VALIDATION.md).

Pendiente:

- Resolver #17 y aprobar su mitigación antes de habilitar login fuera de pruebas.
- Recuperación de contraseña.
- Comprobación privada de sesiones según D-16 y PoC de verificación Rust–Elixir.
- Publicación/consumo de outbox, microcaché e integración con consumidores.
- Inicio de sesión con Google.
- Perfil (nombre visible y foto predefinida) y evento `user.updated`.
- Acordar parámetros definitivos de intentos fallidos; existen presupuestos iniciales por IP y cuenta.
- Usuario de base de datos propio con permisos solo sobre el esquema `auth` (D-13, tarea T-04 en [`docs/PLAN.md`](../../docs/PLAN.md)).

## Estructura

```text
services/auth/
├── Cargo.toml
├── Cargo.lock
├── .env.example
├── migrations/
│   └── ...
├── tests/                # Integración externa: restricciones y outbox
└── src/
    ├── main.rs
    ├── lib.rs
    ├── config.rs
    ├── db.rs
    ├── error.rs
    ├── state.rs
    ├── protection/       # IP del cliente, políticas y limitadores
    │   ├── mod.rs
    │   ├── auth_rate_limiter.rs
    │   ├── client_ip.rs
    │   ├── rate_limit.rs
    │   ├── rate_limit.lua
    │   └── recipient_key.rs
    ├── infrastructure/
    │   ├── mod.rs
    │   └── redis_connection.rs
    ├── tests/            # Pruebas internas del binario
    │   ├── mod.rs
    │   ├── registration.rs
    │   ├── sessions.rs
    │   └── token_support.rs
    ├── dto/
    ├── email/
    ├── handlers/
    ├── ports/
    ├── providers/
    ├── repositories/
    ├── routes/
    ├── security/
    ├── services/
    └── validators/
```

Las pruebas unitarias pequeñas permanecen junto al código que verifican.
`src/tests/` se compila solo con `cfg(test)` y conserva acceso a los módulos
internos; `tests/`, fuera de `src`, contiene las pruebas de integración externas.

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

### Proveedor de correo

`src/config.rs` carga `EMAIL_PROVIDER` desde el entorno: `mailpit` para desarrollo
o `resend` para producción. Si no se define, se utiliza `resend`.
Ambos proveedores implementan `VerificationEmailSender` y comparten la plantilla
HTML y de texto, el idioma y el enlace de verificación.

Para desarrollo, configura `services/auth/.env`:

```env
EMAIL_PROVIDER=mailpit
MAILPIT_SMTP_HOST=localhost
MAILPIT_SMTP_PORT=1025
EMAIL_FROM="Team <noreply@example.com>"
EMAIL_VERIFICATION_URL=http://localhost:8080/auth/verify-email
```

Inicia Mailpit desde la raíz del proyecto:

```bash
docker compose -f infrastructure/compose.yaml up -d mailpit
```

Con estos valores el código envía al puerto SMTP **1025**, sin autenticación ni TLS.
Abre **http://localhost:8025** en el navegador para consultar los correos capturados.
Si auth corre dentro de la red de Compose, utiliza `MAILPIT_SMTP_HOST=mailpit`.
Reinicia auth después de cambiar el proveedor.

Para producción, configura `EMAIL_PROVIDER=resend`, `RESEND_API_KEY`,
`EMAIL_FROM` y una `EMAIL_VERIFICATION_URL` con HTTPS. La clave de Resend
solo se exige cuando se selecciona ese proveedor.

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

Registro/reenvío requieren además Redis accesible al consumir presupuesto y el
proveedor de correo configurado. Para desarrollo con Mailpit se pueden iniciar
las dependencias juntas con `docker compose -f infrastructure/compose.yaml up -d
postgres redis mailpit nginx`. Auth se ejecuta como proceso en el host de esa
red de desarrollo; Compose actualmente no define un contenedor para Auth.

Para verificar el estado del contenedor:

```bash
docker compose -f infrastructure/compose.yaml ps
```

## Ejecutar el Servicio

La VM de Vagrant ya trae Rust estable (con `rustfmt` y `clippy`): `scripts/provision.sh` lo instala con `rustup` para el usuario `vagrant`. En una VM creada antes de este cambio, ejecuta `vagrant provision` desde el equipo anfitrión y vuelve a entrar con `vagrant ssh`.

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

Los ejemplos asumen `SERVER_ADDR=0.0.0.0:3001` y acceso mediante Nginx
en el puerto 8080. El firewall bloquea el acceso directo al puerto 3001.

Health del servicio:

```bash
curl http://localhost:8080/health
```

Respuesta esperada:

```json
{
  "status": "ok"
}
```

Health de base de datos:

```bash
curl http://localhost:8080/health/db
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
  "title": "Database error",
  "status": 503,
  "detail": "The service cannot complete the database operation."
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

Contrato parcial de sesiones y JWKS:

```text
contracts/openapi/auth.yaml
```

Este OpenAPI todavía no describe registro/verificación/reenvío. Sus rutas actuales
`/auth/...` se documentan en [REGISTRATION.md](REGISTRATION.md); completar el contrato
es un pendiente, no una garantía de cobertura de toda la API.

El diseño prevé que los demás servicios validen la firma JWT localmente con JWKS.
La futura comprobación privada del estado de sesión es adicional y tendrá su
propio presupuesto de caché; todavía no está implementada. Ver [SESSIONS.md](SESSIONS.md).

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
