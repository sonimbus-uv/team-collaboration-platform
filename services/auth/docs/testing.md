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
contiene la configuración de CI. Mailpit utiliza `MP_SMTP_DISABLE_RDNS=true` para
que la recepción SMTP de pruebas no dependa de DNS inverso externo, conforme a sus
[opciones de ejecución](https://mailpit.axllent.org/docs/configuration/runtime-options/).
La misma opción se aplica al Mailpit local de Compose. Una ejecución local puede usar contenedores
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
cargo test --locked --test federated_schema
cargo test --locked --test verified_contact_cutover
cargo test --locked --lib google_oidc
cargo test --locked --bin auth-service tests::google:: -- --include-ignored
cargo test --locked --bin auth-service tests::contact_email:: -- --include-ignored
cargo test --locked --bin auth-service tests::security_notifications:: -- --include-ignored
```

Las pruebas cubren transacciones, carreras de refresh y recuperación, fallos
persistentes, expiración, restricciones SQL, consultas read-only, TLS real,
rotación de credenciales, límites distribuidos y ausencia de diagnósticos sensibles.
Los errores de restricciones se comprueban por su código PostgreSQL para evitar
que un fallo de conexión haga pasar una prueba de integridad.

La suite `federated_schema` comprueba la expansión aditiva desde un esquema vacío
y desde fixtures de `0001`–`0006`. Cubre propuestas no exclusivas, versiones de
contacto, límites de códigos, procedencia de sesión, consumo único concurrente,
rollback de finalización y destinatarios de avisos. No prueba login real de Google:
las pruebas de esquema no ejercitan el consentimiento real.

`verified_contact_cutover` aplica `0001`–`0010`, incorpora `0011` y comprueba
conservación de datos, aborto ante direcciones inconsistentes y exclusividad
CITEXT concurrente entre registro, contacto y asignación Google. Las pruebas de
sesiones verifican que una sesión Google sin contacto puede renovarse sin habilitar
login local, y que el refresh no renueva la evidencia de autenticación.

Las pruebas `google_oidc` generan claves RSA desechables mediante OpenSSL y usan
servidores HTTP enlazados a loopback. Cubren firma, algoritmo, emisor, audiencia,
`azp`, tiempos, nonce, autoridad de correo, límites de respuesta, errores sanitizados,
concurrencia, rotación y rechazo de claves vencidas. No requieren credenciales de
Google, PostgreSQL ni Redis. HTTP y endpoints alternativos solo se inyectan desde
el módulo de pruebas; el constructor público utiliza HTTPS y destinos Google fijos.
Estas pruebas no acreditan consentimiento real ni vinculación completos.

`tests::google` añade el recorrido HTTP con proveedor simulado, cookie, PKCE,
renovación de la sesión resultante, consumo concurrente de callback y exchange,
contactos externos, conflictos de exclusividad, cuentas bloqueadas, cancelación y
rollback por fallo al insertar refresh. Las pruebas requieren PostgreSQL desechable;
el límite Redis se omite explícitamente en fixtures, sin cambiar el comportamiento
cerrado del servicio real. Las pruebas de limitación Redis existentes siguen
requiriendo Redis aislado. El consentimiento real de Google y la configuración
nativa de destinos requieren validación operativa aparte.

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

La [validación de integración del 2026-11-14](validation-2026-11-14.md) registra
el entorno, los comandos, Postman y los límites de esa ejecución concreta.

Cada PR registra alcance, comandos ejecutados, dependencias, resultados y pruebas
omitidas con su motivo. Un resultado local no acredita CI remoto, despliegue,
consumidores ni compatibilidad multinodo. Los informes antiguos se conservan
separados de estas instrucciones y no se actualizan como si fueran una guía viva.

Las pruebas de vinculación en `tests::google` cubren sesión autoritativa, evidencia
vencida, reautenticación por contraseña, rechazo tras logout o cambio de credenciales,
idempotencia concurrente, conflictos de proveedor/identidad/contacto y rollback por
fallo de outbox. Verifican que el vínculo no emite tokens, conserva contacto y sesión
y nunca dirige avisos al correo Google sin acreditar. La cuota de reautenticación
por cuenta también se contrasta entre dos instancias usando Redis aislado.
La entrega se comprueba adicionalmente con el worker y Mailpit. Estas pruebas no
acreditan una integración nativa ni consentimiento real de Google.

`tests::contact_email` comprueba HTTP, normalización, idempotencia, vencimiento
conservado en reenvíos, sustitución de propuesta, cinco fallos persistidos, aislamiento
por cuenta, exclusividad concurrente frente al registro, rollback, ausencia de
credenciales nuevas y conservación de evidencia de autenticación. La generación
común conserva un vector de MAC de registro anterior. Los envíos se contrastan
con Mailpit y una prueba bloquea el SMTP para verificar que no retiene locks SQL.

`tests::security_notifications` contrasta claims concurrentes, versión de reserva,
reintentos, agotamiento, destinatario verificado y entrega SMTP real de la plantilla
informativa. No simula garantías externas de exactamente una vez: se valida la
coordinación SQL y la aceptación por el proveedor aislado. Las pruebas de Nginx
incluyen contacto, 4 KiB, errores JSON 413/429 y zonas separadas.


## TLS local del gateway

Dentro de la VM, la preparación y el listener real se verifican sin Auth ni base
de datos:

```bash
python3 scripts/auth/tests/test-gateway-tls-setup.py
python3 scripts/auth/tests/test-auth-jwks-gateway.py --nginx /usr/sbin/nginx --tls
```

Las pruebas utilizan directorios y puertos temporales. Comprueban preservación de
la CA, renovación del servidor, permisos privados, rechazo de nombres ajenos y
material incompleto, TLS 1.2/1.3 y rechazo de certificados sin una CA confiable.
El modo TLS repite las comprobaciones de rutas y límites sobre HTTPS. La CI
incluye ambos modos; no se modifica el almacén de confianza del sistema.
