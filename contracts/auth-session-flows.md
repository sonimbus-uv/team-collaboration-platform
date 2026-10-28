# Login, refresh y logout

**Estado: contrato candidato para desarrollo con mocks, pendiente de revisión
técnica.** Guía de entrada y pendientes para móvil/escritorio:
[auth-clients.md](auth-clients.md). No autoriza habilitar login ni acredita un
despliegue disponible.

Refs #35. [OpenAPI público](openapi/auth.yaml), [perfil](token-profile.md) y
[ADR](../docs/decisions/ADR-007-auth-sessions.md). Implementado en la tercera entrega,
con login deshabilitado por defecto mientras #17 siga abierto; ver
[operación y limitaciones](../services/auth/SESSIONS.md). Rutas bajo `/api/v1/auth`; no
renombra `/auth/register`, `/auth/verify-email` ni `/auth/resend-verification`.
La convergencia del prefijo y contrato de registro/verificación queda en #14.

## Login

Correo/contraseña, `client_type` desktop/mobile y `device_name` opcional (120).
Propuesta: 200 con par de tokens, `token_type: Bearer`, `access_expires_in`
(1–900 s), `session_expires_at`, `sid` y `account_id`. Sin emitir para cuenta
no verificada o no activa. Credencial incorrecta da 401 genérico sin distinguir
cuenta inexistente/contraseña incorrecta. Propuesta: cuenta no habilitada también
usa ese error genérico; política definitiva de enumeración se coordina aparte.

Serializar por cuenta/tipo; reemplazar sesión anterior, registrar revocación
`superseded` (o `expired`) y outbox, crear sesión con límite absoluto de 20 días,
refresh hash y emisión coherente. Fallo transaccional no deja reemplazo parcial.
No responder éxito antes del commit. Las migraciones de sesiones implementan
el reemplazo y el registro transaccional de intención de revocación en outbox.
La publicación y el consumo de esa intención siguen pendientes.

## Refresh

POST con refresh únicamente; no requiere access vigente. Bloquear sesión y token,
comprobar estado de cuenta, sesión y vencimientos. En la misma transacción consumir
actual, guardar hash sucesor y enlazar `replaced_by_token_id`. Nuevo refresh nunca
vive más allá del vencimiento original de sesión; no se modifica ese vencimiento.
Access `exp = min(emisión + 900 s, expiración sesión)`, sin redondeo hacia arriba.
Si no queda un segundo válido, rechazar. Respuesta 200 reemplaza el par anterior.

Si token conocido ya consumido: revocar sesión `refresh_reuse` y guardar outbox,
confirmar esa revocación incluso al devolver 401. No hacer rollback de la
revocación por convertirla en error HTTP. Token desconocido/expirado o sesión
inutilizable devuelve 401 `/problems/invalid-refresh-token`, genérico sin motivo
sensible. Refresh y logout/reemplazo deben serializarse sobre la sesión.

Cliente coordina un único refresh en vuelo por sesión entre tareas/procesos que
compartan llavero; actualiza llavero antes de liberar nuevas renovaciones y
reemplaza access en memoria. No reintentar un refresh consumido. Timeout, conexión
cortada o respuesta ilegible tras envío implican resultado incierto: abandonar
restauración y pedir login; no continuar usando el refresh anterior. Un 503
explícito indica indisponibilidad y no borra automáticamente refresh; el servidor
debe emitirlo solo sin éxito confirmado y documentar no consumo. Si el cliente no
puede distinguir ese resultado de uno incierto, exige login sin reintento.
Fallo al guardar sucesor en llavero: política pendiente, sin persistencia en claro.

## Restauración y logout

Al abrir, leer refresh del llavero y renovar; cerrar ventana/proceso no revoca.
401 definitivo solicita login y retira credencial local no utilizable. 503 conserva
refresh y muestra indisponibilidad; no confundirlo con expiración/revocación.
El comportamiento con llavero inaccesible está pendiente.

Logout propone POST con refresh (incluso access expirado), revoca su sesión y
tokens con motivo `logout` y outbox en transacción. 204 idempotente también para
token desconocido o sesión ya revocada; un token consumido conocido identifica
su familia para revocarla. No aceptar sid arbitrario como credencial. Tras éxito
cliente borra access y refresh. Fallo o desconexión no demuestra revocación;
notificar cierre no confirmado, sin prometer logout remoto offline. UX de reintento
y cierre local sin conexión pendiente de acuerdo. No borra la sesión del otro tipo.

Todas las respuestas de tokens y errores: `Cache-Control: no-store`; HTTPS.
400 entrada inválida, 429 límites públicos (Retry-After), 503 indisponibilidad,
500 fallo interno. Los errores usan Problem Details; no reutilizar cambios de #14.
