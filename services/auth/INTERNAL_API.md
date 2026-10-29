# API interna de sesiones: entrega 4B

Actualización de preparación 2026-10-29: se autorizaron commits locales para
revisión. Las diferencias de formato previas ya se corrigieron. Las tablas de
validación de esta entrega conservan su resultado histórico; la suite conjunta
posterior tiene 120 pruebas correctas y la solución local de #17 se documenta
en [REGISTRATION.md](REGISTRATION.md). Login sigue deshabilitado por defecto.

Decisiones aprobadas por el solicitante el 2026-10-27: contrato HTTP, modelo de
credenciales, convivencia y límites descritos aquí. Reutiliza íntegramente los
tipos, servicio y SELECT de [4A](SESSION_CHECKS.md). No implementa consumidores,
microcaché, WebSocket, RabbitMQ ni recuperación de cuentas.

## Activación y separación

`AUTH_INTERNAL_ENABLED=false` por defecto. Al habilitarlo se requieren dirección,
CIDR de origen permitidos, archivo de credenciales, certificado PEM y clave PEM.
El ejemplo escucha `127.0.0.1:3443` y admite solo loopback. No es una configuración
de acceso remoto: para Core/Files deben aprovisionarse la IP privada, los CIDR
reales de origen, DNS/SAN, certificado y confianza de los clientes.

El router interno solo se entrega al listener TLS; nunca se fusiona con el router
público. Nginx no recibe nuevas rutas ni upstreams. La IP autorizada y las cuotas
por origen usan el socket, normalizando IPv4 mapeada en IPv6. No se aceptan
`X-Real-IP`, `Forwarded` ni `X-Forwarded-For` como identidad de red interna.
El filtro CIDR se aplica antes del handshake y también en la capa HTTP.

La configuración se valida antes de servir cualquiera de los listeners. Fallan
el arranque los valores incompletos, puerto cero, CIDR vacíos o universales,
presupuestos inconsistentes, archivos ilegibles, certificados PEM inválidos y
claves inválidas o distintas al certificado. No hay degradación a HTTP.
La validación local comprueba material PEM y correspondencia de clave; la cadena
de confianza, identidad de destino y vigencia deben verificarlas los clientes TLS.

Credenciales y clave TLS: archivos regulares de hasta 64 KiB, sin symlinks. En
Unix deben pertenecer al usuario de Auth y no tener permisos de grupo/otros
(`0600`/`0400`). En Windows el operador debe imponer las ACL equivalentes;
la comprobación de propietario/permisos Unix no valida ACL de Windows.
La lectura evita seguir symlinks en Unix y limpia los buffers de archivos.
Los ejemplos usan `/run/secrets` fuera del repositorio; los patrones de exclusión
de claves y `secrets/` ya existentes se conservan.

TLS usa Rustls y HTTP/1.1, una solicitud por conexión. Se limita la lectura de
cabeceras, el número de cabeceras a 32 y su buffer a 16 KiB. La respuesta completa
también queda acotada por la suma de presupuestos de cabeceras y solicitud.
Una conexión rechazada antes de HTTP puede cerrarse o recibir una alerta TLS;
no recibe una respuesta de sesión. Los límites HTTP de aplicación se expresan
con los Problem Details siguientes.

## Contrato HTTP aprobado

| Operación | Cuerpo | Respuesta 200 |
|---|---|---|
| POST `/internal/v1/sessions/check` | `{ "sid": UUID, "account_id": UUID }` | Identificadores eco, `active`, `session_expires_at` |
| POST `/internal/v1/sessions/check-batch` | `{ "sessions": [pares] }` | `{ "results": [resultados en orden] }` |

Cuenta activa y verificada, par coincidente, sesión existente, no revocada y
vencimiento estrictamente posterior al instante de evaluación. Inactiva sigue
siendo 200, sin motivo y con vencimiento null. Lotes de 1–100 pares distintos;
mismo sid con otra cuenta es otro par. No hay éxitos parciales ni caché en Auth.

| Estado | `type` de Problem Details | Significado |
|---|---|---|
| 400 | `/problems/malformed-request` | JSON, Content-Type, campos, UUID, lote o query no admitidos |
| 401 | `/problems/invalid-service-credentials` | Credencial ausente, incorrecta, retirada o identidad deshabilitada |
| 403 | `/problems/service-forbidden` | Identidad autenticada sin permiso; el filtro HTTP de red también rechaza con 403 |
| 413 | `/problems/request-too-large` | Cuerpo mayor que el límite configurado |
| 429 | `/problems/service-rate-limited` | Cuota o concurrencia agotada; `Retry-After` entero positivo en segundos |
| 503 | `/problems/session-check-unavailable` | Redis, PostgreSQL, timeout, reserva perdida/expirada o resultado incompleto |

Los errores usan `application/problem+json` sin detalles internos. Las respuestas
de aplicación llevan `Cache-Control: no-store`, `Pragma: no-cache` y
`X-Content-Type-Options: nosniff`; 401 añade `WWW-Authenticate: Bearer`.
Autenticación/autorización preceden a deserialización: una entrada sin credencial
no obtiene detalles del cuerpo. La protección previa puede devolver 429/503 antes
de verificar la credencial. Rutas/métodos distintos no añaden operaciones de sesión.

## Modelo de credenciales

`Authorization: Bearer svc_<base64url sin padding de 32 bytes aleatorios>`.
El token tiene 47 caracteres; es independiente del JWT, refresh y credenciales
AMQP. El hash es SHA-256 de **todo el token ASCII**, incluido `svc_`, representado
por 64 caracteres hexadecimales minúsculos. Se compara con `subtle::ConstantTimeEq`.
SHA-256 es apropiado aquí por la entropía de 256 bits del secreto aleatorio;
las contraseñas humanas mantienen su tratamiento Argon2 existente.

[internal-credentials.example.json](internal-credentials.example.json) es un modelo
con marcadores deliberadamente inválidos: copiarlo sin sustituirlos impide arrancar.
Debe haber exactamente las entradas `core` y `files`; cada una contiene:

- `enabled`: habilitación de la identidad.
- `permissions`: `[]` o `["sessions:check"]`; ninguna otra capacidad.
- `active_sha256`: obligatorio si está habilitada la identidad.
- `previous`: null/ausente o `{ "sha256": "...", "expires_at": "...Z" }`.

No se admite un mismo hash en dos posiciones, ni siquiera entre activa/anterior o
identidades deshabilitadas. Se rechazan campos desconocidos, permisos duplicados,
hashes malformados y configuraciones ambiguas. Los diagnósticos no incluyen
contenido de archivos, secretos ni fragmentos del parser. Los tipos de credencial
no implementan Debug/Serialize. El header se retira antes de pasar al handler;
el router interno no incorpora un logger de solicitudes ni registra errores SQL
con Debug. Las pruebas capturan logs a nivel TRACE y comprueban ausencia de tokens
y hashes de los fixtures; no acreditan políticas de logs de un proxy externo.

Rotación y revocación requieren reinicio, sin recarga dinámica ni tablas:

1. Generar una credencial nueva fuera del repositorio y distribuirla por un canal
   de secretos al servicio correcto. Auth recibe su hash, el consumidor el secreto.
2. Preparar un archivo completo validable con activa nueva y anterior saliente.
   Establecer una fecha UTC absoluta compartida; al arrancar no puede quedar más
   de 15 minutos de convivencia. Reemplazar el archivo de forma atómica.
3. Reiniciar coordinadamente **todas** las réplicas de Auth y cambiar el consumidor
   dentro de esa ventana. La fecha no se recalcula por solicitud ni por reinicio.
4. En el instante exacto `expires_at`, la anterior se rechaza automáticamente.
   Un archivo con anterior ya vencida puede cargarse, pero no la reactiva. Después
   puede retirarse de la configuración. Se requiere un reloj UTC sincronizado.
5. Para revocar anticipadamente, retirar el hash o deshabilitar la identidad y
   reiniciar todas las réplicas. Una réplica que conserve la instantánea anterior
   conserva su política anterior hasta reiniciarse o alcanzar la fecha de caducidad.

No hay secretos de producción generados o instalados por esta entrega.

## Recorrido y protecciones

`main` carga la configuración, valida credenciales/TLS, prepara el pool interno de
solo lectura y enlaza el socket; después arranca ambos servidores. Con la bandera
apagada solo arranca el servidor público habitual. Un fallo de uno de los bucles
de servidores termina el proceso; no se mantiene una exposición parcial silenciosa.

Recorrido: aceptar socket → CIDR y plaza local → handshake TLS → cabeceras → plaza
HTTP local y deadline → cuota previa por IP en Redis → credencial y permiso → JSON
acotado → reserva/cuota compartida por servicio → handler → servicio de 4A →
repositorio y SELECT → liberar reserva → respuesta. Un fallo al liberar o una
reserva inexistente/caducada también devuelve 503. El resultado describe el estado
observado; puede revocarse después y no concede permisos de grupos/archivos.

| Control | Alcance | Valor aprobado por defecto | Agotamiento/fallo |
|---|---|---|---|
| Pares consultados | Redis, por identidad, agregado entre réplicas | Token bucket: capacidad 100, recarga 100 pares/s | 429 + Retry-After; Redis falla: 503 |
| Solicitudes simultáneas autenticadas | Redis, por identidad, agregado entre réplicas | 20 reservas | 429; no espera en cola |
| Vida de reserva | Redis, reloj del servidor | 2 s, al menos doble del presupuesto HTTP | Se poda al admitir; no valida éxito si expiró |
| Solicitudes previas a autenticar | Redis, por IP real, agregado entre réplicas | Token bucket: capacidad 10, recarga 10/s | 429; Redis falla: 503 |
| Conexiones TLS y solicitudes HTTP | Dos semáforos **locales** por proceso | 50 conexiones / 50 solicitudes | Antes de TLS: cierre; en HTTP: 429 |
| Cuerpo JSON | Cada solicitud | 65 536 bytes | 413 |
| Lote | Regla de 4A | 1–100 pares distintos | 400 |
| Solicitud HTTP | Local, incluye cuerpo/auth/Redis/SQL/liberación | 1 s | 503, nunca autorización |
| Adquisición + consulta SQL | Local | 500 ms en conjunto | 503 |
| Conexión Redis / operación Redis | Local | 200 ms / 100 ms | 503; no excepción de correo |
| Handshake TLS / cabeceras | Cada conexión | 300 ms / 1 s | Fallo de transporte, sin resultado de sesión |

Los valores tienen variables `AUTH_INTERNAL_*` en `.env.example`. Un lote cuesta
tantos tokens como pares contiene, una consulta individual cuesta uno. Un lote
duplicado de tamaño permitido consume el coste de sus pares antes de que 4A lo
rechace; los errores de deserialización/tamaño de lote no llegan a esa reserva.
Todos consumen el presupuesto previo por IP. Una cuota no es un máximo de ventana
deslizante: admite el burst del token bucket. La liberación no devuelve tokens.
Una IP/NAT compartida limita también las solicitudes autenticadas a ese presupuesto
previo de 10/s; planificar el uso de lotes y medir el tráfico antes de ajustar valores.

Se reutiliza `RedisConnection` (conexión diferida y timeout) añadiendo un método de
scripts acotado. Un script Lua admite cuota ponderada y reserva de forma atómica,
con `TIME` de Redis, TTL y namespace `auth:internal:v1` independiente de los límites
públicos. Las claves por servicio comparten hash tag para sus operaciones conjuntas.
No se implementa Redis Cluster/HA en esta entrega: todas las réplicas deben usar el
mismo primario lógico, base Redis y valores de límites. No deben usar prefijos
distintos. Las pruebas usan un namespace aleatorio solo bajo `cfg(test)`.

La concurrencia global es un control de **reservas en ese Redis**, no una promesa
de cancelar retroactivamente operaciones ante pausa del proceso o pérdida de estado
del coordinador. Una caída observable impide nuevas consultas. Pérdida de datos,
failover que pierda escrituras o un reinicio de Redis puede reiniciar cuotas/reservas;
antes de desplegar HA debe aprobarse su garantía de persistencia y recuperación.
No se presenta el semáforo local como límite global ni se promete coordinación
linealizable frente a pérdida de datos de Redis.

El pool interno usa el mismo `DATABASE_URL` autoritativo, sin réplica/caché, hasta
`min(concurrencia_por_servicio, 10)` conexiones por proceso. Añade ese presupuesto al
pool público existente: dimensionar conexiones de PostgreSQL al desplegar réplicas.
Solo configura `statement_timeout` y modo read-only en esas conexiones y desactiva
sus logs de sentencias. Un timeout del futuro no garantiza por sí solo que PostgreSQL
haya cancelado trabajo; `statement_timeout` acota también la sentencia en servidor.
No ejecuta migraciones ni modifica cuenta, sesión, refresh u outbox.

No se añaden paquetes nuevos al lockfile: se hacen explícitas cinco dependencias
ya transitivas. `tokio-rustls` aporta TLS, `hyper`/`hyper-util` permiten imponer
timeouts de cabeceras y límites antes de HTTP, `ipnet` valida CIDR y `subtle`
compara los hashes en tiempo constante. El lector PEM ya viene con Rustls.
OpenSSL solo genera certificados desechables en las pruebas, no se invoca en producción.
Referencias de las APIs usadas: [Hyper HTTP/1](https://docs.rs/hyper/latest/hyper/server/conn/http1/struct.Builder.html),
[lector PEM](https://docs.rs/rustls-pki-types/latest/rustls_pki_types/pem/trait.PemObject.html).

## Infraestructura preparada y pruebas aisladas

`infrastructure/firewall/auth-internal.nft.example` es una plantilla loopback-only
para el nuevo puerto; no está incluida en `auth-firewall.service` ni en el script
de aprovisionamiento. Solo se valida con `nft -c`; no se aplica al firewall activo.
No cambia Nginx, Compose, redes existentes ni la confianza de certificados del host.

Desde Linux/VM con Rust, OpenSSL, Docker y puertos de prueba disponibles:

```sh
docker run -d --name sonimbus-auth-4b-pg --label sonimbus.validation=auth-4b \
  -e POSTGRES_USER=test -e POSTGRES_PASSWORD=test -e POSTGRES_DB=auth_test \
  -p 127.0.0.1::5432 postgres:17-alpine
docker run -d --name sonimbus-auth-4b-redis --label sonimbus.validation=auth-4b \
  -p 127.0.0.1::6379 redis:8-alpine
docker port sonimbus-auth-4b-pg 5432
docker port sonimbus-auth-4b-redis 6379
docker exec sonimbus-auth-4b-pg pg_isready -U test -d auth_test
cd services/auth
set -a
. ./.env.example
set +a
# Sustituir PG_PORT y REDIS_PORT por los puertos publicados en loopback.
export DATABASE_URL=postgres://test:test@127.0.0.1:PG_PORT/auth_test
export REDIS_URL=redis://127.0.0.1:REDIS_PORT/0
export AUTH_LOGIN_ENABLED=false AUTH_INTERNAL_ENABLED=false
export CARGO_TARGET_DIR=/tmp/sonimbus-auth-4b-target
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked -- --include-ignored
# Solo 4B: las pruebas arrancan su propio listener aun con la bandera global apagada.
cargo test --locked --bin auth-service internal:: -- --include-ignored
```

Las pruebas crean bases mediante `sqlx::test` (el usuario de pruebas requiere
CREATEDB), sesiones por fixtures sin login, tokens deterministas de prueba y
certificados de un día en directorios temporales. El cliente HTTPS añade esa raíz
solo a su memoria, verifica el hostname y no utiliza `danger_accept_invalid_certs`.
No se cambia el almacén de confianza del anfitrión. Los namespaces Redis de prueba
no comparten ni borran claves del usuario. Las pruebas que requieren Redis están
marcadas como ignoradas para ejecutarse explícitamente con el comando anterior.

Dos pruebas Redis históricas exigen que exista un `.env` aunque sus valores estén
exportados. No sobrescribir el del usuario: si no existe, usar una copia desechable
del proyecto con `.env.example` como `.env`. Las pruebas de 4B no requieren ese archivo.
Al terminar retirar únicamente los dos contenedores identificados arriba y sus
volúmenes desechables, nunca ejecutar `compose down -v` sobre el entorno del usuario.

Cobertura: ambos servicios, permiso ausente, JWT rechazado, rotación/caducidad,
lotes mixtos/duplicados/inválidos, cuerpo, cuotas ponderadas y concurrencia desde
dos instancias independientes, IP real, saturación local, reservas vencidas,
Redis/PostgreSQL caídos, bloqueo SQL, cuerpo lento, HTTPS real, CA no confiable,
hostname distinto, handshake/cabeceras lentos y ausencia de rutas en router público.
La suite previa cubre los flujos públicos y conserva todas las pruebas de 4A.

## Estado inicial, validación final y archivos

Rama `fix/auth-registration-e2e`, commit
`b18146f588c46c033683c0b5278f8cbd89d67e8d`; no cambian referencias Git. No hay
AGENTS.md en el repositorio ni sus ascendentes. Se revisaron convenciones, D-13,
D-16/ADR-007, contratos y el estado real de 4A, configuración, errores, rutas,
confianza de IP, Redis, firewall, Compose y Nginx antes de conectar la nueva API.

Al empezar seguían sin confirmar ocho archivos de 4A. Se conservaron intactos por
hash: `src/lib.rs`, `src/dto/session_checks.rs`,
`src/repositories/session_checks_repository.rs`, `src/services/session_check_service.rs`,
`tests/session_checks.rs`, `SESSION_CHECKS.md` y `SESSIONS.md`. El octavo,
`contracts/session-checks.md`, conserva la aprobación de 4A y añade la de 4B.
No se detectó un fallo funcional bloqueante de 4A ni se reconstruyó esa fase.

| Comprobación | Inicial | Final |
|---|---|---|
| `cargo check --all-targets --locked` | Correcta | Correcta |
| `cargo clippy --all-targets --locked -- -D warnings` | No ejecutada como baseline | Correcta |
| `cargo test --locked -- --include-ignored` | 92 correctas, 0 fallos/ignoradas | **113 correctas, 0 fallos/ignoradas** |
| Formato de archivos Rust de 4B | Aún no existían/cambiaban | Correcto en los nueve archivos del alcance |
| `cargo fmt --check` global | Fallos en seis archivos previos | Exactamente los mismos fallos |
| `git diff --check` | Dos líneas con espacios finales en `src/lib.rs` de 4A | Mismas dos líneas; ninguna nueva |
| Nginx real en puertos temporales | No ejecutado como baseline | Sintaxis, JWKS, presupuestos previos y ambas rutas internas 404 correctos |
| `nft -c` de la plantilla interna | No existía | Correcto; no aplicada |
| Sintaxis JSON de OpenAPI/ejemplo | No ejecutada como baseline | Correcta; no es certificación completa de OpenAPI |

Las seis diferencias de formato previas son `src/lib.rs`,
`src/services/session_check_service.rs`, `src/handlers/sessions.rs`,
`src/repositories/refresh_tokens_repository.rs`, `tests/revocation_outbox.rs` y
`tests/session_constraints.rs`. No se reformatearon para ocultar el estado inicial.
La suite final se divide en 12 pruebas de biblioteca, 93 del binario y 8 externas;
las 21 nuevas pertenecen a 4B. No se detectaron regresiones.

Windows no tiene Cargo en PATH. Se inició la VM existente sin aprovisionar y se
usaron PostgreSQL/Redis desechables publicados solo en loopback, sin alterar `.env`.
La descarga inicial falló por DNS de `static.crates.io`; se retiraron dependencias
innecesarias y las verificaciones finales se ejecutaron con `--offline` y el
lockfile. Durante las pruebas se corrigieron dos expectativas del fixture TLS:
certificado de servidor con `CA:FALSE` y rechazo de HTTP que puede devolver una
alerta TLS binaria. No se cambió la verificación de confianza para hacerlas pasar.
No se ejecutó CI remoto ni se aplicó configuración al despliegue real.

Nuevos archivos de 4B:

- `src/internal/{mod.rs,config.rs,credentials.rs,limits.rs,admission.lua,server.rs}`.
- `src/tests/internal_api.rs`.
- `INTERNAL_API.md`, `internal-credentials.example.json`.
- `infrastructure/firewall/auth-internal.nft.example` (ruta desde la raíz).

Modificados por 4B, además del trabajo previo identificado:

- `src/main.rs`, `src/config.rs`, `src/infrastructure/redis_connection.rs`.
- `Cargo.toml`, `Cargo.lock`, `.env.example`, `README.md`.
- `contracts/session-checks.md`, `contracts/openapi/auth-internal.yaml` y
  `scripts/test-auth-jwks-gateway.py` (rutas desde la raíz).

Propuesta de commits para revisión, sin preparar ni ejecutar:

1. `feat(auth): expose protected internal session checks over TLS`: dependencias,
   `src/internal/`, integración de `main.rs`/`config.rs` y método de scripts Redis.
   Dejar el hunk que conecta el módulo de pruebas HTTP para el siguiente commit.
2. `test(auth): verify internal TLS, credentials and shared limits`:
   `src/tests/internal_api.rs`, hunk de inclusión en `src/internal/mod.rs` y
   `scripts/test-auth-jwks-gateway.py`.
3. `docs(auth): document approved 4B contract and deployment`:
   `.env.example`, ejemplo JSON, este documento, README, contratos y plantilla nft.

Los cambios se dejan sin preparar ni confirmar; no se ejecuta ningún commit ni
se continúa automáticamente con otra entrega.

## Requisitos de los consumidores futuros y pendientes

Core/Files deberán verificar firma, emisor, audiencia y vencimiento de JWT y asociar
`sid` a `sub`. Una futura microcaché positiva admite hasta 10 segundos desde el inicio
monotónico de la consulta, descontando la latencia y acotada a JWT/sesión; no se
renueva al leer ni se reutiliza vencida ante fallos. Una revocación conocida debe
impedir instalar una respuesta tardía favorable. Si falta una comprobación necesaria,
la operación protegida falla temporalmente con 503, sin revocar ni borrar refresh.
Esto no implementa esos consumidores. La caché JWKS de cinco minutos es independiente.

Pendiente de despliegue: IP/puerto y CIDR reales, DNS/SAN, emisión/distribución y
renovación de certificados, archivos de credenciales y reinicios de todas las
réplicas, dimensionamiento y mediciones de límites; política HA/persistencia de
Redis antes de atribuirle garantías durante pérdida de estado. No se aplicó red,
firewall, confianza ni aprovisionamiento. No se acredita interoperabilidad Elixir.

`AUTH_LOGIN_ENABLED` conserva false por defecto y en el ejemplo. El estado local
de #17 sigue sin mitigación de recuperación aprobada; esta API no resuelve el
registro con correo ajeno ni autoriza habilitar login. No se modificó GitHub.

Siguiente entrega recomendada: revisar 4A/4B y validar un despliegue privado aislado
con los responsables de Core/Files y la PoC Rust–Elixir antes de integrar consumidores.
RabbitMQ y los consumidores permanecen fuera de esta entrega.
