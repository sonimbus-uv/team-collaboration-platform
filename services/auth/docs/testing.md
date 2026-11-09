# Pruebas de Auth

Esta guía contiene procedimientos reproducibles. Los resultados de ejecuciones
anteriores pertenecen al [archivo histórico](history.md), no constituyen un
resultado actual de CI y no deben usarse como una cantidad fija de pruebas.

## Dependencias y aislamiento

La suite completa requiere Rust con rustfmt y Clippy, PostgreSQL de pruebas con
permiso `CREATEDB`, Redis y Mailpit. Las pruebas TLS requieren OpenSSL; las
comprobaciones Unix de permisos se ejecutan en Linux. La VM aprovisionada aporta
el entorno de desarrollo descrito en [desarrollo](development.md).

`sqlx::test` crea una base migrada por prueba. Los casos Redis usan claves únicas;
Mailpit usa destinatarios ficticios y únicos. El entorno de pruebas no debe
apuntar a bases, credenciales, cuotas o buzones de producción ni de uso personal.
No se necesita cargar el `.env` completo de Auth para las pruebas de conexión Redis.

Las dependencias de referencia son `postgres:17-alpine`, `redis:8-alpine` y
`axllent/mailpit:v1.29.7`. El [workflow Auth](../../../.github/workflows/auth.yml)
contiene la configuración de CI. Una ejecución local puede usar contenedores
dedicados con nombres únicos y puertos publicados solo en loopback; los puertos
efectivos deben sustituirse en el siguiente ejemplo antes de ejecutarlo:

```bash
export DATABASE_URL='postgres://test:test@127.0.0.1:PG_PORT/auth_test'
export REDIS_URL='redis://127.0.0.1:REDIS_PORT/0'
export AUTH_TEST_MAILPIT_SMTP_PORT='SMTP_PORT'
export AUTH_TEST_MAILPIT_API_URL='http://127.0.0.1:MAILPIT_HTTP_PORT'
export AUTH_RECIPIENT_HMAC_SECRET='ci-only-secret-0123456789abcdef0123456789abcdef'
export AUTH_REGISTRATION_CODE_HMAC_SECRET='ci-only-registration-code-key-not-for-deployment'
export CARGO_TARGET_DIR="$HOME/.cargo-target/auth-service"
```

Los secretos de este ejemplo son fixtures públicos exclusivos de pruebas, no
material válido para un despliegue. La disponibilidad se comprueba con
`pg_isready`, `redis-cli ping` y la API HTTP de Mailpit antes de ejecutar Cargo.
La limpieza retira únicamente contenedores, volúmenes y archivos creados por esa
ejecución; no utiliza `compose down -v` sobre el entorno habitual.

## Suite Rust

Desde `services/auth`, con las dependencias aisladas disponibles:

```bash
cargo fmt --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked -- --include-ignored
```

`--include-ignored` ejecuta también los casos que requieren Redis o Mailpit; su
ausencia en una ejecución parcial no acredita el funcionamiento completo.
La biblioteca puede comprobarse sin iniciar el binario ni cargar claves de
producción mediante `cargo test --locked --lib`.

Los filtros permiten ejecutar un área tras un cambio localizado:

```bash
cargo test --locked --bin auth-service tests::registration:: -- --include-ignored
cargo test --locked --bin auth-service tests::passwords:: -- --include-ignored
cargo test --locked --bin auth-service internal:: -- --include-ignored
cargo test --locked --test session_checks
cargo test --locked --test session_constraints
cargo test --locked --test revocation_outbox
```

Las pruebas cubren transacciones, carreras de refresh y recuperación, fallos
persistentes, expiración, restricciones SQL, consultas read-only, TLS real,
rotación de credenciales, límites distribuidos y ausencia de diagnósticos sensibles.
Los errores de restricciones se comprueban por su código PostgreSQL para evitar
que un fallo de conexión haga pasar una prueba de integridad.

## Scripts de desarrollo y gateway

Desde la raíz del repositorio:

```bash
bash -n scripts/auth/dev/setup-auth-dev.sh scripts/auth/dev/start-auth-dev.sh
python3 scripts/auth/tests/test-auth-dev-setup.py
python3 scripts/auth/tests/test-auth-jwks-gateway.py --nginx /usr/sbin/nginx
```

La prueba de preparación usa OpenSSL y archivos temporales, con Compose simulado,
sin modificar Docker ni la base real. Comprueba confianza, permisos, credenciales
distintas y conservación de configuración existente. El caso de gateway usa Nginx
real y backend desechable; valida rutas, cabeceras y límites, no autenticación
real del backend ni interoperabilidad con Core/Files.

## Registro y Postman

Con un proceso Auth de prueba, claves desechables, login habilitado únicamente
en ese entorno y Mailpit aislado:

```bash
python3 scripts/auth/tests/test-auth-registration-code.py \
  --base-url http://127.0.0.1:AUTH_PORT \
  --mailpit-url http://127.0.0.1:MAILPIT_HTTP_PORT --confirm-isolated
```

El script comprueba el escenario de intentos independientes y el ciclo de sesión
sin imprimir credenciales. HTTP loopback corresponde solo al entorno aislado.
Las colecciones Postman se ejecutan según [desarrollo](development.md#arranque);
validar la sintaxis de sus scripts no equivale a ejecutar las colecciones.

## Evidencia de validación

Cada PR registra alcance, comandos ejecutados, dependencias, resultados y pruebas
omitidas con su motivo. Un resultado local no acredita CI remoto, despliegue,
consumidores ni compatibilidad multinodo. Los informes antiguos se conservan
separados de estas instrucciones y no se actualizan como si fueran una guía viva.
