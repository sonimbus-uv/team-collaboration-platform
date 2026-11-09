# Validaciones históricas de Auth

Este archivo conserva evidencia de entregas anteriores. Sus cantidades de pruebas,
ramas, limitaciones y estados de funcionalidades corresponden a aquellas revisiones;
no describen el código vigente ni acreditan el estado actual de CI o del despliegue.
Los procedimientos actuales se mantienen en [pruebas](testing.md) y el estado
funcional en el [README](../README.md). Los textos originales completos permanecen
en el historial Git de los documentos sustituidos.

## Documentos sustituidos

| Documento anterior | Ubicación vigente |
|---|---|
| `DEVELOPMENT.md` | [Desarrollo](development.md) |
| `REGISTRATION.md`, `SESSIONS.md`, `SESSION_CHECKS.md`, `PASSWORDS.md` | [Arquitectura](architecture.md), [configuración](configuration.md) y contratos enlazados |
| `TOKENS.md`, `INTERNAL_API.md`, `RATE_LIMITING.md` | [Operación](operations.md) y [configuración](configuration.md) |
| `VALIDATION.md`, `REGISTRATION_VALIDATION.md` | Evidencia seleccionada en este archivo; originales completos en Git |

## Revisión documental de octubre de 2026

### Comprobaciones ejecutadas

Entorno Linux de la VM Vagrant. Dependencias desechables y aisladas: PostgreSQL
`17-alpine`, Redis `8-alpine` y Mailpit `v1.27`, publicados solo en loopback en
puertos dinámicos. Configuración por variables temporales y claves Ed25519
generadas exclusivamente para esta comprobación. No se modificó el `.env` local
ni se usó la base de desarrollo para los datos del recorrido HTTP.

Desde `services/auth`, con las variables de ese entorno:

```sh
cargo fmt --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked -- --include-ignored
cargo build --locked
```

Resultado: formato, compilación y Clippy estricto correctos. **83 pruebas
correctas, cero fallos y cero ignoradas**: 8 de biblioteca, 72 del binario y
3 de integración externa. Incluye los tests Redis y comprobaciones Unix de claves.

Desde la raíz:

```sh
python3 scripts/auth/tests/test-auth-jwks-gateway.py --nginx /usr/sbin/nginx
```

Resultado: prueba existente con Nginx real correcta; sintaxis, JWKS GET/HEAD,
caché, presupuestos separados y 404. Ese script utiliza un backend stub.

Adicionalmente se ejecutó un recorrido con el binario real de Auth y copias
temporales de la configuración Nginx del repositorio, cambiando puertos,
upstream, rutas de archivos y logs. El proxy confiable de esa comprobación fue
loopback. Se verificó:

1. Health de PostgreSQL/Redis y JWKS a través de Nginx.
2. Registro 201 con correo recibido en Mailpit; registro duplicado 409.
3. HEAD del enlace 405 sin consumo; reenvío durante cooldown 202 sin otro correo.
4. Reenvío elegible 202 y recepción de un segundo correo. Para evitar la espera
   de un minuto se envejeció `created_at` únicamente en la base desechable.
5. El enlace original siguió funcionando mediante GET después del reenvío.
6. Tras confirmar, el enlace alternativo y la reutilización devolvieron 400.
7. Registro de otra cuenta y confirmación POST 200 con `email_verified=true`.
8. PostgreSQL confirmó exactamente dos cuentas verificadas.
9. Una ráfaga produjo 429 con `Retry-After`.

Los procesos, contenedores y claves temporales de esa comprobación se limpiaron
al terminar. Se instaló Nginx en la VM para ejecutar estas comprobaciones.

También se comprobó la sintaxis del firewall con `nft -c`, la unidad
`auth-firewall.service` activa y la tabla instalada: permiso desde `br-auth` /
`172.30.250.2` al puerto 3001 y rechazo para las demás conexiones a ese puerto.
Esta lectura no sustituye una prueba de tráfico por la topología desplegada;
el recorrido aislado utilizó otros puertos y no acredita ese bloqueo con paquetes.

## Registro por intento y código, octubre de 2026

### Validación ejecutada

Windows no dispone de Cargo en PATH. Se utilizó la VM existente sin aprovisionar,
con PostgreSQL 17, Redis 8 y Mailpit v1.29.7 desechables en puertos loopback.
No se modificó `.env` ni se usaron bases, claves o correos reales.

| Control | Inicial | Final |
|---|---|---|
| Compilación de suite / `cargo check --all-targets --locked --offline` | Correcta | Correcta |
| `cargo test --locked --offline -- --include-ignored` | 113 correctas | **120 correctas**, 0 fallos/ignoradas |
| Clippy todos los targets, `-D warnings` | No se ejecutó como baseline en esta entrega | Correcto |
| `cargo build --locked --offline` | No requerido como baseline independiente | Correcto; binario usado en smoke real |
| `cargo fmt --check` global | Seis archivos con diferencias previas | Salida idéntica a la inicial |
| `git diff --check` | Dos espacios finales previos en `src/lib.rs` | Mismos dos; ninguno añadido |
| Proceso real Auth + SMTP/API Mailpit | No ejecutado como baseline | Registro A/B, rechazo legacy, login, refresh y logout correctos |
| Nginx temporal real | No repetido como baseline | Sintaxis, JWKS, presupuestos y 404 internos correctos |
| OpenAPI | Sin revisión adicional inicial | JSON y 44 referencias locales correctos |
| Script smoke Python / workflow YAML | No existían/cambiaban | Sintaxis Python y lectura YAML correctas |

La primera ejecución inicial falló en dos tests Redis históricos por variables
JWT ausentes, no por cambio de código. Al exportar valores ficticios y conservar
las bases aisladas, la suite inicial pasó con 113. Durante desarrollo se corrigió
una expectativa del fixture JWKS (faltaba `must-revalidate`) y una condición
redundante del test de concurrencia detectada por Clippy. No se cambió el contrato
de JWKS ni se relajó ninguna comprobación para hacer pasar esas pruebas.

La suite final tiene 12 pruebas de biblioteca, 100 del binario y 8 externas;
el incremento neto es siete. Se sustituyeron las pruebas del protocolo retirado
por las del nuevo flujo, sin presentar las antiguas expectativas como vigentes.
Incluye: vigencia/consumo/bloqueo, código de otro intento, cooldown y reenvíos,
contador persistido, presupuesto compartido de correo, fallo de entrega,
rollback SQL tras fallo inyectado, dos confirmaciones concurrentes del mismo
intento y de intentos distintos, cuenta heredada pendiente, cuenta verificada
inmutable, SMTP real, logs TRACE sin códigos/contraseñas/MAC y comprobación de
que el envío puede adquirir el bloqueo de registro desde otra conexión.
La vigencia y cooldown se prueban ajustando timestamps en bases aisladas, sin
esperas reales de diez minutos. No se acredita recuperación durante pérdida de
datos de PostgreSQL/Redis ni entrega real de Resend, ni ejecución de CI remoto.

Los seis archivos de formato previo son `src/lib.rs`,
`src/services/session_check_service.rs`, `src/handlers/sessions.rs`,
`src/repositories/refresh_tokens_repository.rs`, `tests/revocation_outbox.rs`
y `tests/session_constraints.rs`. Se formatearon solo archivos afectados por #17.

## Tokens y JWKS

### Registro histórico de la segunda entrega

Los resultados y referencias de este apartado corresponden a esa entrega,
no al inventario actual ni al estado remoto de sus PR. La validación vigente
se ejecuta con los comandos anteriores y se registra en [VALIDATION.md]().

Resultado local de esa entrega: formato y `cargo check --all-targets --locked`
correctos; Clippy estricto de biblioteca y Clippy de todos los targets correctos
(con las dos advertencias previas del binario). Ocho pruebas de biblioteca,
42 pruebas del binario y la prueba adicional de router completo pasan: 51 en
total. Tres pruebas Redis permanecen ignoradas y siete pruebas SQLx dependientes
de PostgreSQL no se ejecutaron. Permisos/symlinks Unix esperan ejecución en CI
Linux. La prueba de gateway pasa con Nginx Windows 1.30.6 oficial; ni ese resultado
ni las pruebas Rust acreditan interoperabilidad Elixir o CI remoto exitoso.

Base: rama dependiente de `70cfb9b`, PR #44 todavía abierto al comenzar. Un PR de
esta entrega debe comparar contra la rama documental hasta integrarla; después
revisar su base sin reescribir historia ni duplicar cambios. Próxima entrega:
resolver #17 y enumeración antes de habilitar login; luego sesiones transaccionales.

## Sesiones públicas

### Registro histórico de validación de la tercera entrega

Los resultados siguientes pertenecen a esa entrega. No describen la ejecución
actual ni prueban el estado remoto de CI/PR. Ver [VALIDATION.md]()
para las comprobaciones de la reorganización y revisión documental.

Antes de editar: formato, compilación y 51 pruebas locales de fase 2 correctos;
dos advertencias heredadas de PasswordHasher, tres pruebas Redis ignoradas y
siete SQLx aún no ejecutadas por falta de servidor. Una caché temporal Cargo
incompleta falló al localizar fuentes; usar una caché temporal nueva resolvió
ese problema de entorno sin cambios al código heredado ni a Cargo.lock.

Resultado de fase 3: 77 pruebas correctas (8 biblioteca, 66 binario y 3 de
migraciones), cero fallos en la suite ejecutada. PostgreSQL 17.11 se descargó
y arrancó exclusivamente en un directorio temporal, con autenticación de prueba
y puerto loopback efímero. SQLx aplicó las migraciones en bases aisladas.
Se detuvo ese servidor al finalizar, sin borrar ni reiniciar datos del usuario.
Cuatro pruebas Redis ignoradas y dos anteriores dependientes de Redis no se
ejecutaron; quedan para CI. Los tests Unix de permisos esperan Linux.

Formato, compilación y Clippy estricto de todos los targets correctos; las
advertencias previas desaparecen al usar ahora verificación de contraseñas.
Nginx real 1.30.6 valida las rutas nuevas, presupuestos y JWKS. Referencias y
enlaces documentales comprobados; `git diff --check` correcto. No se detectaron
regresiones en las comprobaciones ejecutadas. No se declara CI remoto exitoso.

Pendientes al cerrar aquella entrega: #17 y su contrato de recuperación/enumeración;
límites operativos definitivos; pruebas Redis completas y Linux; limpieza/retención; publicación
RabbitMQ, API interna, consumidores e interoperabilidad. Siguiente entrega:
resolver #17 con contrato y pruebas antes de habilitar login, sin ampliar esta
entrega a recuperación completa ni integración con los compañeros.

## Consulta interna de dominio

### Inspección y validación

Base: rama `fix/auth-registration-e2e`, commit
`b18146f588c46c033683c0b5278f8cbd89d67e8d`, árbol inicialmente limpio.
Se contrastaron estándares, decisiones D-13/D-16, ADR-007, contratos, código de
cuentas/registro/verificación, login/refresh/logout, rutas y las cuatro migraciones.

Confirmado previamente: PostgreSQL autoritativo, propiedad de datos, sesiones
absolutas, revocación y operación de lectura. El contrato ya describía cuenta activa
y verificada, eco/orden y fallo completo; la revisión los ratificó junto con
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

## API privada HTTPS

### Estado inicial, validación final y archivos

Rama `fix/auth-registration-e2e`, commit
`b18146f588c46c033683c0b5278f8cbd89d67e8d`; no cambian referencias Git. No hay
D-16/ADR-007, contratos y el estado real de 4A, configuración, errores, rutas,
confianza de IP, Redis, firewall, Compose y Nginx antes de conectar la nueva API.

Al empezar seguían sin confirmar ocho archivos de 4A. Se conservaron intactos por
hash: `src/lib.rs`, `src/dto/session_checks.rs`,
`src/repositories/session_checks_repository.rs`, `src/services/session_check_service.rs`,
`tests/session_checks.rs`, `SESSION_CHECKS.md` y `SESSIONS.md`. El octavo,
`contracts/auth/internal-api.md`, conserva la aprobación de 4A y añade la de 4B.
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
- `contracts/auth/internal-api.md`, `contracts/openapi/auth-internal.yaml` y
  `scripts/auth/tests/test-auth-jwks-gateway.py` (rutas desde la raíz).
