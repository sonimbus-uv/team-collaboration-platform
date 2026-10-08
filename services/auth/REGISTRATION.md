# Registro y verificación de correo

Configurar `RESEND_API_KEY`, `EMAIL_FROM`, `EMAIL_VERIFICATION_URL` y
`MAX_CONCURRENT_HASHES` usando `.env.example`. El remitente debe estar autorizado
en Resend. La URL de verificación apunta al endpoint público del Auth Service
`GET /auth/verify-email`. Resend añade el token como parámetro de query.
Se exige HTTPS salvo para localhost. No se necesita un cliente web.

## API

- `GET /auth/verify-email?token=...`: confirma mediante el mismo servicio que POST
  y devuelve HTML: 200 en éxito, 400 para un enlace no utilizable y 500/503 ante
  errores internos o indisponibilidad. HEAD devuelve 405 sin consumir el token.
  Las respuestas HTML incluyen `no-store`, `no-referrer` y CSP restrictiva.
  El límite compartido puede devolver 429 con HTML y `Retry-After`.
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
- `POST /auth/resend-verification`: `{"email":"user@example.com"}`.
  Devuelve 202 también para cuentas inexistentes, verificadas o en cooldown.
  Una emisión nueva invalida los tokens pendientes anteriores. Hay un mínimo de
  60 segundos entre emisiones para la misma cuenta. Un fallo de envío devuelve 500
  con mensaje genérico; se puede reintentar tras el cooldown.

Los tokens contienen 32 bytes aleatorios y vencen a las 24 horas; la base guarda
solo su digest SHA-256. No se registran tokens ni contraseñas en los logs.
Si el envío inicial falla, la cuenta permanece creada y el registro devuelve
`verification_email_sent=false`: el cliente debe ofrecer reenvío, no repetir el registro.
Si el proceso se interrumpe después de persistir, también se recupera con reenvío.
No hay una cola de entrega ni reintentos automáticos.

Argon2 se ejecuta en `spawn_blocking`, con un máximo configurable de trabajos
simultáneos (4 por defecto). El servicio rechaza saturación con 429 y Retry-After.
Los endpoints de autenticación comparten un límite fijo de 120 solicitudes por
minuto por proceso. No es un límite por IP ni compartido entre réplicas; el gateway
todavía no tiene configuración y debe añadir ese control cuando se despliegue.

La validación acepta direcciones ASCII de tipo dot-atom y dominios DNS; no admite
locales entre comillas ni direcciones internacionales sin convertir a ASCII.
Validar el formato no prueba la existencia del buzón.

## Validación

Desde `services/auth`:

```sh
cargo fmt --check
cargo check
cargo test -- --skip integration_tests
```

Para las pruebas completas, usar un PostgreSQL de desarrollo con un usuario que
pueda crear bases (`CREATEDB`) y sea propietario de la base indicada en
`DATABASE_URL` (o tenga permiso para crear esquemas en ella). `sqlx::test` crea una
base aislada por prueba, aplica las migraciones y la elimina al terminar.

```sh
cargo test
```

Las pruebas de integración usan un emisor falso: no mandan correos ni necesitan
una clave Resend. Cubren HTTP de registro/confirmación, duplicados concurrentes,
rollback ante un token duplicado, reenvío/cooldown, confirmación concurrente,
reutilización, vencimiento, cambio de correo, fallo de envío y errores genéricos.

Proveedor implementado conforme a la [API de envío de Resend](https://resend.com/docs/api-reference/emails/send-email).
