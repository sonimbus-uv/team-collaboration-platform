# Registro con código: solución local de #17

Flujo y parámetros aprobados el 2026-10-28, pendiente de revisión técnica.
Contrato vigente: [registration.md](../../contracts/registration.md) y
[OpenAPI público 0.2.0](../../contracts/openapi/auth.yaml). Decisión y límites:
[ADR-008](../../docs/decisions/ADR-008-registration-attempt-codes.md).

El registro conserva correo y contraseña en la aplicación: crea un intento
independiente, envía código y confirma ese identificador+código. No pide volver
a establecer contraseña en una página web, no activa mediante enlaces y no
emite sesiones al confirmar. `AUTH_LOGIN_ENABLED=false` sigue siendo el valor
predeterminado, incluso después de aplicar esta solución local.

## Implementación

Handler `handlers/auth.rs` → `AuthService`/`EmailVerificationService` →
`AccountsRepository`/`RegistrationAttemptsRepository`. Este último ahora solo gestiona
intentos de registro; ya no confirma tokens heredados. La tabla nueva es
`auth.registration_attempts` (migración 0005). No se modificaron migraciones
anteriores ni se purgaron datos heredados.

Argon2id corre en `spawn_blocking` con semáforo compartido con login. Se almacena
un hash por intento; el código se protege con HMAC-SHA256 independiente ligado
a propósito, UUID y versión. Nunca se almacena el código original. El UUID es
impredecible, pero no basta para confirmar. La MAC se compara en tiempo constante.

La creación/reenvío/confirmación bloquean por correo, luego cuenta y luego intento.
La confirmación verifica de nuevo dentro de la transacción. Solo una gana:
instala el hash correspondiente, verifica cuenta, consume intento e invalida los
demás. Las cuentas antiguas pendientes pueden reclamarse con un intento nuevo;
las verificadas o bloqueadas no se sobrescriben. Sesiones heredadas se revocan
con `password_changed`, utilizando el trigger de outbox existente. Un fallo SQL
revierte toda activación. Fallos de código se confirman antes de devolver 400.

El correo sale después del commit. Fallo/timeout conserva intento y comunica
`verification_email_sent=false`. El cliente podrá reenviar tras 60 segundos,
hasta tres veces sin prolongar los diez minutos originales ni reiniciar fallos.
Puede existir un envío en vuelo cuando otro intento gana; recibirlo no vuelve
a habilitar el intento invalidado. No hay cola ni reintentos automáticos.

Registro/reenvío requieren Redis. Confirmación mantiene la excepción previa de
disponibilidad ante caída Redis: el presupuesto de cinco fallos vive en SQL.
Se mantienen las cuotas IP y se comparte el presupuesto por destinatario entre
registro y reenvío. Saturación de cupos puede denegar temporalmente nuevos
intentos; no elimina los existentes. El modelo no promete disponibilidad frente
a un atacante que mantenga ocupado el cupo del destinatario.

## Configuración y secreto

Se añade `AUTH_REGISTRATION_CODE_HMAC_SECRET`, requerido al iniciar, sin default.
Generar al menos 32 bytes aleatorios y aprovisionarlos fuera del repositorio,
igual en todas las réplicas y distinto de `AUTH_RECIPIENT_HMAC_SECRET`. Rotarlo
inutiliza códigos pendientes; se requiere iniciar otro intento. No se implementa
convivencia de claves. No copiar valores de prueba a producción.

Se elimina `EMAIL_VERIFICATION_URL`: ya no se construyen enlaces. Mailpit y Resend
reutilizan el puerto de correo existente; el contenido indica finalidad, plazo,
no compartir código e ignorar registros no solicitados. No contiene recursos
externos ni enlaces de activación. `.env.example` documenta los nuevos requisitos
y la cuota por destinatario aprobada de 10/600 s.

## Pruebas aisladas con Mailpit

Usar PostgreSQL/Redis/Mailpit desechables y puertos loopback diferentes de los
servicios del usuario. Nunca aplicar limpieza masiva ni `compose down -v` sobre
su entorno. SQLx requiere usuario CREATEDB y crea bases por prueba.

1. Exportar las variables de `.env.example` en un shell de prueba y sustituir
   `DATABASE_URL`, `REDIS_URL`, secretos y rutas de claves por recursos aislados.
   No sobrescribir `.env` del usuario. Las pruebas de claves usan fixtures propios;
   los tests Redis históricos requieren que exista un `.env` válido aunque
   los valores efectivos se exporten por proceso.
2. Para SMTP real, exportar `AUTH_TEST_MAILPIT_SMTP_PORT` y
   `AUTH_TEST_MAILPIT_API_URL=http://127.0.0.1:PUERTO_HTTP_MAILPIT`.
3. Desde `services/auth`, ejecutar:

```sh
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked -- --include-ignored
# Solo registro, incluidas pruebas Redis/Mailpit explícitas:
cargo test --locked --bin auth-service tests::registration:: -- --include-ignored
```

Para probar el proceso real, aprovisionar claves de prueba conforme a [TOKENS.md](TOKENS.md),
arrancar Auth con esas variables, SMTP Mailpit y listener loopback. Habilitar
login únicamente mediante variable del proceso de prueba aislado, sin editar
defaults ni `.env`. Ejecutar desde la raíz:

```sh
python3 scripts/test-auth-registration-code.py \
  --base-url http://127.0.0.1:PUERTO_AUTH \
  --mailpit-url http://127.0.0.1:PUERTO_HTTP_MAILPIT --confirm-isolated
```

El script crea una dirección ficticia única, reproduce atacante A/dueño B,
consulta códigos solo en Mailpit y prueba login/refresh/logout, sin imprimir
credenciales. HTTP loopback es exclusivo de la prueba; el despliegue real requiere
HTTPS. También se puede registrar manualmente y leer el código en la UI de
Mailpit, enviando su identificador con el código por JSON. No ponerlo en URLs.

## Pendientes antes de habilitar login

Revisión del cambio incompatible y clientes; auditoría explícita de cuentas ya
verificadas por el flujo anterior; configuración segura del secreto y políticas
de retención/limpieza de intentos expirados; revisión de enumeración (409 sigue
revelando cuentas verificadas/bloqueadas). El algoritmo no sanea datos antiguos.
Forgot/reset password, Google, Core/Files y publicación RabbitMQ no se implementan.
El issue permanece abierto; la entrega local no equivale a aprobación operativa.
