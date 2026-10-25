# Registro y verificación de correo

Configurar PostgreSQL, Redis, `AUTH_RECIPIENT_HMAC_SECRET`, correo y claves JWT
según [README.md](README.md), [RATE_LIMITING.md](RATE_LIMITING.md) y
[TOKENS.md](TOKENS.md), usando `.env.example` como referencia. Las claves JWT
son obligatorias para arrancar incluso si solo se usan registro y verificación.
`EMAIL_PROVIDER=mailpit` utiliza SMTP local; `resend` requiere `RESEND_API_KEY`
y un remitente autorizado. Ambos proveedores añaden el token al enlace construido
por Auth, cuya base es `EMAIL_VERIFICATION_URL` y apunta a `GET /auth/verify-email`.
Se exige HTTPS salvo HTTP para `localhost` o `127.0.0.1`. No se necesita un cliente web.

## API

- `GET /auth/verify-email?token=...`: confirma mediante el mismo servicio que POST
  y devuelve HTML: 200 en éxito, 400 para un enlace no utilizable y 500/503 ante
  errores internos o indisponibilidad. HEAD devuelve 405 sin consumir el token.
  Las respuestas HTML incluyen `no-store`, `no-referrer` y CSP restrictiva.
  Los límites por IP de Nginx o Auth pueden devolver 429 con HTML y `Retry-After`.
  Un segundo clic devuelve 400, aunque la primera confirmación haya tenido éxito.

La confirmación GET cambia estado para permitir verificar con un solo clic.
Los escáneres automáticos de correo también pueden consumir el enlace; este flujo
no demuestra una confirmación humana explícita. Para exigirla, sería necesario
mostrar un formulario y confirmar mediante POST.

La página invita a volver a Sonimbus. Cuando escritorio y móvil soporten deep links,
se podrá añadir un botón fijo `sonimbus://email-verified?status=success` únicamente
tras confirmar. No debe transportar tokens ni aceptar destinos arbitrarios. La app
debe consultar Auth; `status=success` no constituye prueba de verificación.
El gateway y la observabilidad deben redactar la query del enlace y evitar registrar
tokens. No añadir recursos de terceros a la página de verificación.

- `POST /auth/register`: `{"email":"user@example.com","password":"ValidPass123!"}`.
  Devuelve 201 con `user_id`, `email`, `email_verified`, `status` y
  `verification_email_sent`. Una cuenta recién creada tiene `email_verified=false`.
  Cuenta, credencial y token se insertan atómicamente. Un correo duplicado devuelve 409.
- `POST /auth/verify-email`: `{"token":"<token recibido>"}`.
  Devuelve 200 con el estado de la cuenta. Token inválido, vencido, invalidado,
  utilizado o asociado a otro correo devuelve 400. Consumir el token y marcar el
  correo como verificado es una transacción; solo una confirmación concurrente gana.
  La respuesta reutiliza `RegisterResponse`; `verification_email_sent=false` en
  esta confirmación no indica un fallo de entrega, sino que aquí no se envía correo.
- `POST /auth/resend-verification`: `{"email":"user@example.com"}`.
  Devuelve 202 también para cuentas inexistentes, verificadas o en cooldown.
  Los enlaces anteriores conservan su vencimiento original. Confirmar uno invalida
  los demás pendientes. Hay un mínimo de 60 segundos entre emisiones para la misma
  cuenta. Un fallo del proveedor se registra y mantiene el 202; el token ya está
  persistido y se puede reintentar tras el cooldown. Presupuesto por destinatario
  agotado o no disponible también mantiene 202, sin emitir otro token. Los errores
  de PostgreSQL sí pueden producir 500/503. Un 202 no garantiza envío ni entrega.

Los tokens contienen 32 bytes aleatorios, representados como 64 caracteres
hexadecimales, y vencen a las 24 horas; la base guarda SHA-256 del texto del token.
El flujo de envío no registra tokens ni contraseñas. Los errores de PostgreSQL
en `ApiError::Database` sí se registran con Debug: no se debe asumir que toda
diagnosis SQL está redactada. Ver las limitaciones de registros en RATE_LIMITING.md.
Si el envío inicial falla, la cuenta permanece creada y el registro devuelve
`verification_email_sent=false`: el cliente debe ofrecer reenvío, no repetir el registro.
Si el proceso se interrumpe después de persistir, también se recupera con reenvío.
No hay una cola de entrega ni reintentos automáticos.

Argon2 se ejecuta en `spawn_blocking`, con un máximo configurable de trabajos
simultáneos (4 por defecto). El servicio rechaza saturación con 429 y Retry-After.
No hay un límite global fijo de 120 solicitudes por minuto en Auth. Nginx tiene
presupuestos por IP/operación y Auth utiliza buckets de Redis por IP/operación,
además del presupuesto por destinatario en reenvíos elegibles. GET y POST de
verificación comparten bucket. Redis se conecta al primer uso del limitador o de
`/health/redis`, no al construirlo. Ante fallo de Redis por IP, registro y reenvío
responden 503; verificación continúa. Un rechazo explícito por presupuesto agotado
produce 429. Ver valores y alcance en [RATE_LIMITING.md](RATE_LIMITING.md).

Los handlers de registro, confirmación POST y reenvío utilizan el extractor
`Json<T>` de Axum sin adaptar sus rechazos. JSON mal formado, tipos incompatibles
o Content-Type incorrecto pueden producir respuestas nativas de Axum (400/422/415),
no Problem Details. Tampoco rechazan campos JSON desconocidos. Las rutas de
sesión tienen un contrato distinto, documentado en [SESSIONS.md](SESSIONS.md).

La validación acepta direcciones ASCII de tipo dot-atom y dominios DNS; no admite
locales entre comillas ni direcciones internacionales sin convertir a ASCII.
Validar el formato no prueba la existencia del buzón.

## Validación

Desde `services/auth`:

```sh
cargo fmt --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
# Solo pruebas de la biblioteca de tokens, sin PostgreSQL/Redis:
cargo test --lib --locked
```

Para las pruebas completas, usar un PostgreSQL de desarrollo con un usuario que
pueda crear bases (`CREATEDB`) y sea propietario de la base indicada en
`DATABASE_URL` (o tenga permiso para crear esquemas en ella). `sqlx::test` crea una
base aislada por prueba, aplica las migraciones y la elimina al terminar.

```sh
cargo test --locked -- --include-ignored
```

La suite completa requiere también Redis, `.env` válido y las variables JWT de
configuración, incluso para los tests de conexión Redis. Las pruebas con emisor
falso no envían correo ni cargan archivos JWT de despliegue. No sobrescribir un
`.env` existente para ejecutar pruebas; se pueden pasar variables temporales.

`src/tests/registration.rs` cubre reenvíos no elegibles y concurrentes, cooldown,
presupuesto agotado/no disponible, fallo de envío preservando el enlace anterior,
confirmación HTTP GET/POST, reutilización, HEAD sin consumo, indisponibilidad de
Redis durante confirmación y JWKS sin dependencias. Estas pruebas no sustituyen
una comprobación de entrega real mediante Mailpit ni demuestran por sí solas todos
los escenarios de registro, expiración, cambio de correo o rollback por colisión.

Proveedor implementado conforme a la [API de envío de Resend](https://resend.com/docs/api-reference/emails/send-email).
