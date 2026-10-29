# Consulta autoritativa de sesiones: entrega 4A

Implementación interna de Auth, sin handlers, rutas ni endpoints nuevos. La entrega
termina en esta capa; no implementa 4B. Decisiones aprobadas expresamente por el
solicitante el 2026-10-26 tras revisar las propuestas de `contracts/session-checks.md`.
No modifica vigencias, login, refresh, logout, migraciones ni outbox.

## Contrato aprobado para 4A

- Entrada individual: `sid` y `account_id`, ambos UUID obligatorios. Serde rechaza
  UUID malformados, campos ausentes, null y campos desconocidos; no se exige una
  versión particular de UUID. Los tipos Rust contienen UUID ya analizados.
- Entrada por lotes: `SessionCheckBatchRequest { sessions }`, de 1 a 100 pares.
  Se rechaza el lote completo si está vacío, excede 100 o repite un par. Un mismo
  sid con otra cuenta es un par diferente: se evalúa y no acredita la cuenta real.
  Un elemento malformado impide deserializar todo el lote antes de llamar al servicio.
- Una sesión es activa únicamente si existe, pertenece a la cuenta indicada,
  `revoked_at IS NULL`, `expires_at > observed_at`, y la cuenta tiene estado
  `active` y correo verificado. Igualdad con el vencimiento significa inactiva.
  `locked`, `suspended`, `deleted` o correo sin verificar producen inactiva.
- Cada resultado contiene solo `sid`, `account_id`, `active`, `session_expires_at`.
  Los identificadores son eco de la entrada; el vencimiento UTC RFC 3339 conserva
  fracciones de segundo y solo se devuelve para sesiones activas. Inactiva siempre
  devuelve null, sin motivos ni datos de otra cuenta. No devuelve credenciales.
- El lote conserva orden y cardinalidad, un resultado por par. Una consulta
  individual usa el mismo recorrido de evaluación que un lote de un elemento.
- `InvalidInput` es un error de dominio de validación del lote. Los errores de
  deserialización ocurren antes del servicio. `Database(sqlx::Error)` conserva
  separados los fallos de infraestructura, incluidos timeout y pool cerrado.
  `IncompleteResult` rechaza respuestas con cardinalidad o asociación incorrectas.
  Ninguno produce `active=true`, resultados negativos fabricados ni éxito parcial.
  Esta entrega no asigna estados HTTP a esos errores.

## Recorrido y SQL

API de biblioteca: `auth_service::session_check_types` y
`auth_service::session_check_service::SessionCheckService::new(pool)`; métodos
`check(request)` y `check_batch(request)`. Los archivos siguen las carpetas DTO,
repositorio y servicio existentes. Se exportan desde `lib.rs`, como el componente
de tokens, para permitir pruebas de integración sin iniciar el binario ni requerir
claves de firma, correo o Redis. El repositorio es privado a la biblioteca; el
binario y el router no se modifican. No se añaden traits ni dependencias.

El servicio valida el lote antes del acceso DB. El repositorio enlaza dos arrays
UUID parametrizados y usa `unnest ... WITH ORDINALITY`, `LEFT JOIN` de sesiones por
sid **y cuenta**, y `LEFT JOIN` de cuentas. Una sola sentencia SELECT devuelve
también pares inexistentes. No ejecuta una consulta por elemento. El servicio
verifica la asociación completa antes de construir resultados e interpreta las
condiciones de actividad usando el mismo reloj para todos los elementos.

`statement_timestamp()` de PostgreSQL aporta el instante de inicio de la sentencia;
las tablas se leen con la instantánea MVCC de esa sentencia. No se abre una
transacción adicional ni se usa `FOR UPDATE`: una sola lectura conjunta evita
mezclar una lectura de cuenta con otra lectura independiente de sesión. Los
índices primarios existentes de sesiones y cuentas permiten las búsquedas por
UUID; no hay necesidad demostrada de un índice o migración adicional.

La consulta es solo lectura: no actualiza actividad, no revoca vencidas, no limpia,
no consume refresh, no modifica cuentas ni genera eventos. Las pruebas la ejecutan
con conexiones configuradas como read-only y comparan el contenido completo de
cuentas, sesiones, refresh y outbox antes y después.

El pool debe apuntar al PostgreSQL **primario autoritativo de Auth**, nunca a una
réplica con retraso. No hay caché ni fallback. El constructor no puede comprobar la
topología del despliegue: el responsable de conexión debe preservar esa condición.
Ningún otro servicio debe acceder directamente al esquema `auth`.

El resultado describe lo observado, no reserva autorización: una revocación o un
cambio de cuenta pueden ocurrir después. No garantiza que una sesión siga activa
al recibir la respuesta ni decide permisos de grupos, canales o archivos. Tampoco
valida JWT; el consumidor futuro deberá verificarlo por separado. No se fija un
timeout nuevo en 4A; los errores del pool/servidor se propagan sin resultados.

## Inspección y validación

Base: rama `fix/auth-registration-e2e`, commit
`b18146f588c46c033683c0b5278f8cbd89d67e8d`, árbol inicialmente limpio.
No se encontraron AGENTS.md en el repositorio ni sus ascendentes.
Se contrastaron estándares, decisiones D-13/D-16, ADR-007, contratos, código de
cuentas/registro/verificación, login/refresh/logout, rutas y las cuatro migraciones.

Confirmado previamente: PostgreSQL autoritativo, propiedad de datos, sesiones
absolutas, revocación y operación de lectura. El contrato ya describía cuenta activa
y verificada, eco/orden y fallo completo; el solicitante los ratificó junto con
1–100, duplicados e inválidos para 4A. Los parámetros operativos de red siguen
siendo propuestas. La arquitectura y el estándar general aún mencionan sesiones
Auth en Redis; el código real las mantiene en PostgreSQL. Los contextos históricos
del ADR y del plan describen capacidades ausentes entonces; hoy existen sesiones
transaccionales, refresh/logout y outbox. No se interpretan como código faltante.

En Windows no hay Cargo en PATH. Docker Desktop no responde; la VM Vagrant estaba
apagada. Se inició sin provisionar. El target de compilación de la carpeta
compartida falló por un archivo generado de serde ausente; repetir con
`CARGO_TARGET_DIR=/tmp/sonimbus-auth-4a-target` en Linux resolvió el problema.
No se borraron artefactos ni datos previos.

Estado inicial en VM, antes de compilar los cambios: `cargo check --all-targets
--locked` correcto; `cargo test --locked` correcto, 79 pruebas aprobadas y 4 ignoradas
que requieren Redis. Las pruebas SQLx crean bases separadas por prueba. El formato
inicial ya fallaba en cuatro archivos: handlers/sessions.rs,
repositories/refresh_tokens_repository.rs, tests/revocation_outbox.rs y
tests/session_constraints.rs (rutas bajo Auth).
No se reformatean archivos ajenos a 4A.

Resultado final en VM: `cargo check --all-targets --locked` y
`cargo clippy --all-targets --locked -- -D warnings` correctos. Suite completa con
`cargo test --locked -- --include-ignored`: **92 pruebas correctas, cero fallos y
cero ignoradas** (12 biblioteca, 72 binario, 8 integración externa). Se añaden
nueve pruebas respecto a las 83 existentes. La suite final ejecuta también las
cuatro antes ignoradas al disponer de Redis aislado. Formato de los cinco archivos Rust de 4A correcto;
`cargo fmt --check` global conserva exactamente los cuatro archivos con fallos
previos. `git diff --check` correcto. No se detectaron regresiones.

La primera ejecución ampliada falló en dos pruebas Redis antiguas por falta de
`AUTH_JWT_ISSUER`; eran requisitos de configuración, no fallos de consulta. Al
exportar las variables de `.env.example` y sobrescribir las URLs con los servicios
desechables pasó la suite completa, sin cambiar `.env`. Durante el desarrollo se
corrigió un timeout del fixture demasiado corto para establecer la conexión; el
test final reserva la conexión antes de comprobar la saturación y usa un segundo
de presupuesto. El timeout de sentencia se provoca con un bloqueo en su base
aislada, no esperando un vencimiento real de sesión.

Cobertura nueva: cuatro pruebas unitarias de validación, vencimiento exacto con
reloj fijo y precisión de microsegundos, y defensa ante respuestas incompletas;
cinco pruebas externas de lote mixto de 100 pares equivalente a individual, todas
las condiciones negativas, cambios posteriores de cuenta, ausencia de escrituras,
validación antes de SQL, pool cerrado, timeout de adquisición, error SQL y timeout
real de sentencia PostgreSQL. Los timeouts de prueba son locales a sus pools;
no constituyen parámetros de despliegue. Los fixtures crean sesiones directamente
sin habilitar login.

## Reproducción aislada

Ejecutar en Linux/VM desde `services/auth`, con Docker disponible. Los nombres de
contenedor siguientes deben estar libres; los puertos se asignan dinámicamente.
No copiar `.env` ni usar una base de desarrollo. Credenciales exclusivamente de prueba:

```sh
docker run -d --name sonimbus-auth-4a-pg --label sonimbus.validation=auth-4a \
  -e POSTGRES_USER=test -e POSTGRES_PASSWORD=test -e POSTGRES_DB=auth_test \
  -p 127.0.0.1::5432 postgres:17-alpine
docker run -d --name sonimbus-auth-4a-redis --label sonimbus.validation=auth-4a \
  -p 127.0.0.1::6379 redis:8-alpine
docker port sonimbus-auth-4a-pg 5432
docker port sonimbus-auth-4a-redis 6379
# Sustituir PG_PORT y REDIS_PORT por los puertos impresos; esperar pg_isready.
# La suite histórica ampliada requiere configuración completa:
set -a
. ./.env.example
set +a
export DATABASE_URL=postgres://test:test@127.0.0.1:PG_PORT/auth_test
export REDIS_URL=redis://127.0.0.1:REDIS_PORT/0
export AUTH_LOGIN_ENABLED=false
export AUTH_RECIPIENT_HMAC_SECRET=4a-test-only-secret-0123456789abcdef0123456789abcdef
export CARGO_TARGET_DIR=/tmp/sonimbus-auth-4a-target
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked -- --include-ignored
# Solo pruebas nuevas:
cargo test --locked --lib session_check
cargo test --locked --test session_checks
# Formato solo del alcance:
rustfmt --check --edition 2021 --config skip_children=true src/lib.rs \
  src/dto/session_checks.rs src/repositories/session_checks_repository.rs \
  src/services/session_check_service.rs tests/session_checks.rs
```

Usar un usuario de prueba capaz de crear bases para `sqlx::test`. Sus fixtures y
migraciones quedan en bases aisladas; la prueba de fallo SQL altera únicamente su
propia base. Dos pruebas Redis históricas llaman además `dotenvy::dotenv().expect`:
para la suite completa debe existir un `.env` en esa copia de trabajo (o un
ascendente). No se cambió el existente durante esta validación; se exportaron las
variables del ejemplo y las URLs aisladas antes de ejecutarla. Si no existe `.env`,
ejecutar las pruebas nuevas, que no lo requieren, o usar una copia de trabajo
desechable con `.env.example` como `.env`. No sobrescribir configuración del usuario.
Al terminar se pueden eliminar exclusivamente los contenedores
desechables identificados arriba con `docker rm -f -v` y sus nombres exactos;
nunca ejecutar `compose down -v` sobre la infraestructura del usuario.

## Pendientes para 4B

- Aprobar rutas y traducción HTTP; los OpenAPI internos siguen siendo diseño de red.
- Credenciales mínimas: identidad, permiso `sessions:check`, credencial activa y
  anterior, solapamiento, rotación, revocación, distribución y almacenamiento;
  decidir reinicio frente a recarga y su coherencia entre réplicas.
- Listener HTTPS, certificados y separación respecto al acceso público.
- Límites HTTP de cuerpo, cuotas por identidad/par, concurrencia y presupuestos
  temporales; separar límites locales de límites compartidos entre réplicas.
  Un semáforo local no constituye un límite global.
- Resolver Redis para límites distribuidos y comportamiento ante caída; presupuestos
  de conexión/SQL/total y cancelación, sin convertir errores en inactividad.
- Los valores históricos 1 s/300 ms, 64 KiB, tasas y concurrencias continúan como
  propuestas; aprobar 100 pares para 4A no aprueba el resto de parámetros.

#17 permanece sin mitigación aprobada según el estado local; no se consultó ni
modificó GitHub. `AUTH_LOGIN_ENABLED` sigue false por defecto y en el ejemplo.
4A no implementa recuperación ni autoriza habilitar login. No hay cambios de Core,
Files, clientes, gateway, firewall, publicación de outbox o broker.

## Cambios y commits propuestos para revisión (sin ejecutarlos)

1. `feat(auth): add authoritative read-only session checks`: `src/lib.rs`,
   `src/dto/session_checks.rs`, `src/repositories/session_checks_repository.rs`,
   `src/services/session_check_service.rs` (incluye pruebas unitarias).
2. `test(auth): verify authoritative session checks in isolated PostgreSQL`:
   `tests/session_checks.rs`.
3. `docs(auth): record approved 4A contract and validation`: este documento,
   `SESSIONS.md`, `../../contracts/session-checks.md`.

No se prepara ni confirma ningún cambio, no se cambian ramas ni referencias y no
se continúa automáticamente con 4B.
