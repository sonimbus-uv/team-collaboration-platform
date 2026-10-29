# Entrega local para revisión: registro por intento y código (#17)

Actualización de preparación 2026-10-29: el solicitante autorizó organizar los
cambios en commits locales pequeños, sin push. El informe siguiente conserva el
estado de la entrega original. Después se corrigieron las seis diferencias de
formato: `cargo fmt --check` y `git diff --check` pasan; ya no son un bloqueo de
formato para CI. La ejecución del solicitante confirmó 120 pruebas correctas.

Fecha: 2026-10-28. Rama `fix/auth-registration-e2e`, HEAD inicial/final
`666df09228f00f80e16e1b84d6bafffeb8a09707`. No se preparan archivos ni se crean
commits; no se cambian ramas/refs, clientes, servicios consumidores ni GitHub.
Solo se leyó el issue #17 y sus comentarios: abierto, sin comentarios al consultar.
No se encontraron AGENTS.md en el repositorio ni sus ascendentes.

## Decisiones y comportamiento

El solicitante aprobó explícitamente los tres grupos propuestos: parámetros,
contrato HTTP y protección/compatibilidad. Ver
[ADR-008](../../docs/decisions/ADR-008-registration-attempt-codes.md) y
[registration.md](../../contracts/registration.md). No hay decisión pendiente
para la mecánica implementada. Se conserva login deshabilitado por defecto.

El registro no crea cuenta: crea intento con contraseña propia y código enviado
al correo, protegido mediante HMAC independiente y UUID/versión/propósito.
Confirmar ID+código instala solo ese hash. Los intentos coexisten sin expulsión;
una confirmación válida consume uno e invalida los demás de forma atómica.
Los fallos se confirman antes del error HTTP y los reenvíos no renuevan su cupo
ni la vigencia. Correo después del commit, sin transacción abierta.

El escenario de #17 se reproduce expresamente con Argon2id real: atacante A y
dueño B registran el mismo correo en intentos distintos; B confirma su propio
intento, B autentica, A no autentica, el código de A no modifica la cuenta.
Solicitudes adicionales no invalidan el intento del dueño. El límite puede
negar nuevas solicitudes temporalmente si se llena, limitación aprobada.

No hay cambio de contraseña en web ni recuperación de cuentas verificadas.
La migración 0005 solo añade tabla/índice; nunca se ejecutó sobre datos reales.
GET antiguo informa 410, HEAD 405; POST con token y reenvío por correo no activan
nada. Una nueva confirmación válida puede reclamar una cuenta antigua pendiente;
las ya verificadas por el flujo vulnerable requieren auditoría separada.

## Validación ejecutada

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

## Inventario frente al trabajo previo

Se conservaron los cambios locales iniciales de 4A/4B y el commit de documentación
previo. Servicio/repositorio de consultas 4A y Lua de admisión conservan sus hashes.
No se modificó la lógica de sesiones, refresh, JWT/JWKS ni límites internos 4B.
`src/tests/internal_api.rs` solo adapta el repositorio de registro y el secreto
explícito del constructor usado por su fixture.

Nuevos archivos de esta entrega:

- `migrations/0005_registration_attempts.sql`.
- `src/repositories/registration_attempts_repository.rs`.
- `src/security/registration_code.rs`.
- `REGISTRATION_VALIDATION.md`.
- Desde raíz: `contracts/registration.md`,
  `docs/decisions/ADR-008-registration-attempt-codes.md`,
  `scripts/test-auth-registration-code.py`.

Archivos modificados por esta entrega (rutas relativas a Auth salvo indicación):

- `src/main.rs`, `src/config.rs`, `src/dto/auth.rs`, `src/error.rs`,
  `src/handlers/auth.rs`, `src/routes/mod.rs`.
- `src/repositories/{accounts_repository.rs,mod.rs}` y
  `src/services/{account_service.rs,email_verification_service.rs}`.
- `src/security/mod.rs`, `src/ports/email_verification.rs`,
  `src/protection/auth_rate_limiter.rs`.
- `src/providers/{mailpit.rs,resend.rs}` y
  `src/email/{mod.rs,verification_page.rs,verification_template.rs}`.
- `src/tests/{registration.rs,sessions.rs,internal_api.rs}`.
- `.env.example`, `README.md`, `REGISTRATION.md`, `RATE_LIMITING.md`,
  `SESSIONS.md`, `VALIDATION.md`.
- Desde raíz: `.github/workflows/auth.yml`, `contracts/auth-clients.md`,
  `contracts/openapi/{README.md,auth.yaml}`,
  `docs/decisions/ADR-007-auth-sessions.md`.

Retirados: `src/repositories/one_time_tokens_repository.rs` (se elimina el camino
de confirmación antiguo) y `src/email/brand.rs` (plantilla anterior de enlace y
logo externo sustituida por correo de código sin recursos externos).
No hay dependencias nuevas: se reutilizan HMAC/SHA-256, rand, uuid, zeroize,
Argon2, SQLx y proveedores existentes. Los cambios locales de Cargo.toml/lock
siguen perteneciendo a 4B, no a #17.

## Configuración real y pendientes

La inspección limitada a nombres y valores no sensibles de `.env` encontró:
`AUTH_LOGIN_ENABLED` y `AUTH_INTERNAL_ENABLED` ausentes (defaults false), cuota
por destinatario aún 3/3600 s y secreto nuevo de códigos ausente. No se imprimieron
otros valores ni se modificó ese archivo. Antes de arrancar hay que aprovisionar
el secreto independiente y revisar la cuota frente al ejemplo aprobado 10/600 s.

La configuración CI local añade servicio Mailpit y variables exclusivamente de
prueba para ejecutar los tests ignorados explícitamente. No se ejecutó ni cambió
ningún workflow remoto. Los seis problemas de formato previos siguen siendo un
pendiente para CI global; no se ocultaron con reformateos ajenos al alcance.

Antes de habilitar login: revisión del equipo, auditoría/saneamiento aprobado de
cuentas verificadas heredadas, política de enumeración y cupos, secreto en todas
las réplicas, retención de intentos expirados y adaptación JavaFX/Flutter.
Pruebas manuales con Mailpit y comandos en [REGISTRATION.md](REGISTRATION.md).
Forgot/reset, Google, consumidores, PoC Rust–Elixir y RabbitMQ siguen pendientes.

## Propuesta de commits, sin ejecutarlos

Primero revisar/separar el trabajo previo de 4A/4B: la prueba de ciclo completo
comprueba la sesión mediante el servicio 4A ya presente en el árbol local.

1. `feat(auth): bind registration codes to independent attempts`: migración,
   repositorios, HMAC, servicios, DTO/handlers/rutas, configuración y proveedores/
   plantillas; adaptaciones mínimas de fixtures necesarias para compilar.
2. `test(auth): cover registration takeover races and native session lifecycle`:
   pruebas de registro, script smoke, fixture de límites, Mailpit/variables CI.
3. `docs(auth): publish attempt-bound registration contract for review`: OpenAPI
   0.2.0, contrato, ADR-008, ejemplos e informes/guías actualizados.

Son agrupaciones de revisión, no commits preparados. La solución se deja para
inspección; no se declara cerrado #17 ni se habilita login por haber pasado tests.
