# Registro con código ligado al intento — contrato candidato 0.2.0

Flujo y parámetros aprobados el 2026-10-28 para resolver #17. Pendiente de revisión
técnica e integración de clientes; login sigue deshabilitado por defecto.
Ver [ADR-008](../docs/decisions/ADR-008-registration-attempt-codes.md) y
[OpenAPI](openapi/auth.yaml). Cambio incompatible respecto a enlace/token.

## Registro

`POST /auth/register`, JSON `{"email":"person@example.invalid","password":"ExamplePass123!"}`.
Normaliza correo y valida la política actual de contraseña; Argon2id fuera del
runtime con concurrencia limitada. Devuelve 201 con `registration_attempt_id`
(UUID v4), `expires_at` UTC y `verification_email_sent` booleano. No devuelve
cuenta, sesión, hash ni código. La aplicación conserva el identificador de SU
intento junto al flujo de registro, no lo sustituye por el correo.

La contraseña queda ligada solo a ese intento, no a otros para el mismo correo.
Hasta cinco intentos pendientes utilizables pueden coexistir diez minutos, sin
expulsión. Una cuenta antigua no verificada no impide iniciar un intento nuevo.
Una cuenta verificada/bloqueada mantiene 409; esto conserva enumeración conocida.

## Confirmación

`POST /auth/verify-email`, JSON con `registration_attempt_id` y `code` string de
ocho dígitos (preservar ceros iniciales). 200 devuelve `user_id`, `email`,
`email_verified=true`, `status` y `verification_email_sent=false` (no se envía
correo al confirmar). No inicia sesión ni devuelve access/refresh.

Solo se instala el hash del intento confirmado. Se consume el intento y se
invalidan los demás del mismo correo en la transacción de activación. Después
de confirmar, otro registro o código nunca puede sustituir esa contraseña.

Código incorrecto o con formato inválido para un intento existente consume un
fallo persistido. Cinco fallos bloquean ese intento, incluso si después llega
el código correcto. Se mantienen los fallos después de reenvíos.

## Reenvío

`POST /auth/resend-verification`, JSON con `registration_attempt_id`, sin correo.
202 devuelve los mismos campos de intento que registro. Máximo tres reenvíos,
intervalo mínimo de 60 segundos, nuevo código y vencimiento original intacto.
El código anterior queda invalidado. Los reenvíos de otros intentos no afectan
al propio. Si el intento no existe, está consumido/bloqueado/vencido, devuelve
400 genérico; si agotó reenvíos o está en cooldown, 429.

`verification_email_sent=true` significa aceptación del proveedor, no entrega.
Si falla, devuelve false, conserva el intento y habilita el próximo reenvío
solo tras cooldown; no se amplía el plazo ni se restauran códigos previos.
Si el intento vence, solicitar uno nuevo. No reintentar registro automáticamente
tras un resultado incierto; podría crear otro intento sujeto al cupo.

## Límites, errores y clientes

- Código/intento: diez minutos absolutos desde creación, cinco fallos totales.
- Hasta cinco intentos pendientes utilizables por correo; sin invalidación al crear otros.
- Presupuesto compartido registro/reenvío: capacidad diez, recarga completa en
  600 s por destinatario, configurable mediante límites existentes.
- Presupuestos IP existentes: registro 6/60 s, reenvío 3/60 s, verificación 20/60 s.
- JSON máximo 4096 bytes. Campos desconocidos, UUID o JSON inválidos: 400.
- Confirmación no utilizable: 400 genérico, sin distinguir razón.
- Cuenta registrada: 409. Límites: 429 con `Retry-After` entero.
- Tamaño: 413. PostgreSQL/Redis necesarios no disponibles: 503.
- Hash/worker fallido: 500. Errores con `application/problem+json` y `about:blank`,
  siguiendo el formato existente. Respuestas de registro/verificación/reenvío
  incluyen `Cache-Control: no-store`, incluso errores de entrada.

Ante Redis caído se conserva la política previa: registro/reenvío fallan con
503; verificación puede continuar porque sus cinco intentos se controlan en
PostgreSQL. Un rechazo explícito de la cuota IP sí produce 429. Códigos,
contraseñas y MAC no se incluyen en logs, métricas ni URLs.

JavaFX/Flutter deben cambiar DTO y pantalla: registro → conservar ID → solicitar
código recibido → confirmar ID+código. Reenvío utiliza ese mismo ID. Un 201 no
significa cuenta creada/verificada. No enviar contraseña de nuevo al confirmar.
No interpretar un enlace antiguo ni el ID solo como prueba de verificación.

## Enlaces y datos anteriores

GET `/auth/verify-email` responde 410 informativo, ignorando query/token y sin
cambiar datos; HEAD 405. POST `{"token":...}` y reenvío `{"email":...}` ya no
son admitidos. Los enlaces anteriores no activan nada. La migración no purga
cuentas ni tokens reales. Una confirmación nueva válida puede reclamar una
cuenta antigua no verificada con su propio hash; cuentas verificadas por el
flujo anterior requieren auditoría aparte, sin corrección automática.

Antes de habilitar login: revisión técnica y de enumeración, auditoría de
cuentas heredadas, aprovisionamiento/rotación del secreto en todas las réplicas,
comunicación del cambio incompatible y pruebas de clientes en entorno aislado.
Forgot/reset para cuentas verificadas sigue pendiente.
