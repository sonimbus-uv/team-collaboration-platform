# API pública de Auth

Esta guía reúne los acuerdos de registro, sesiones y contraseñas para móvil,
escritorio y otros consumidores públicos. El [OpenAPI público](../openapi/auth.yaml)
define rutas, esquemas y estados HTTP. Las secciones siguientes explican su
semántica sin acreditar un despliegue disponible ni habilitación de producción.

## Alcance y pendientes

El backend implementa registro por intento y código, login, refresh, logout,
JWT/JWKS y cambio y recuperación de contraseña. Login y contraseñas permanecen
deshabilitados por defecto y requieren activación explícita. La integración de
clientes, la auditoría de cuentas heredadas y la revisión de enumeración y cuotas
siguen pendientes antes de habilitación productiva.

Las operaciones públicas usan `/api/v1/auth`; JWKS conserva
`/.well-known/jwks.json`. La API interna pertenece exclusivamente a Core/Files,
usa un listener separado y no distribuye credenciales a clientes nativos.
Las URL de ejemplo de OpenAPI son ficticias; cada integración requiere acordar
el entorno real y las credenciales de prueba.

## Reglas para los clientes nativos

El cliente conserva access en memoria y refresh en el almacén seguro del sistema
operativo según el [perfil de tokens](tokens.md). La coordinación de renovaciones
y el manejo de respuestas inciertas se definen en [sesiones](#sesiones).
Los logs y los ejemplos compartidos excluyen credenciales y cuerpos sensibles.

## Coordinación y revisión

El desarrollo inicial puede utilizar mocks y ejemplos ficticios. Las discrepancias
con la implementación requieren revisión, sin convertir propuestas históricas en
garantías. Un cambio incompatible se comunica a móvil y escritorio y sigue las
[reglas de contratos](../README.md#mantenimiento). El tratamiento de un llavero
inaccesible, el fallo al guardar un sucesor y el cierre local sin conexión
requieren acuerdos adicionales de cliente.

La [documentación del servicio](../../services/auth/README.md) concentra arquitectura,
configuración, operación y pruebas. Las decisiones se conservan en
[ADR-007](../../docs/decisions/ADR-007-auth-sessions.md) y
[ADR-008](../../docs/decisions/ADR-008-registration-attempt-codes.md).

## Registro y verificación

Contrato de registro candidato 0.2.0, con cambio incompatible aprobado el
2026-10-28 respecto a enlaces de activación. La revisión técnica e integración
siguen pendientes. Las operaciones se definen en
[auth-registration.yaml](../openapi/auth-registration.yaml).

### Registro

`POST /api/v1/auth/register`, JSON `{"email":"person@example.invalid","password":"ExamplePass123!"}`.
Normaliza correo y valida la política actual de contraseña; Argon2id fuera del
runtime con concurrencia limitada. Devuelve 201 con `registration_attempt_id`
(UUID v4), `expires_at` UTC y `verification_email_sent` booleano. No devuelve
cuenta, sesión, hash ni código. La aplicación conserva el identificador de SU
intento junto al flujo de registro, no lo sustituye por el correo.

La contraseña queda ligada solo a ese intento, no a otros para el mismo correo.
Hasta cinco intentos pendientes utilizables pueden coexistir diez minutos, sin
expulsión. Una cuenta antigua no verificada no impide iniciar un intento nuevo.
Una cuenta verificada/bloqueada mantiene 409; esto conserva enumeración conocida.

### Confirmación

`POST /api/v1/auth/verify-email`, JSON con `registration_attempt_id` y `code` string de
ocho dígitos (preservar ceros iniciales). 200 devuelve `user_id`, `email`,
`email_verified=true`, `status` y `verification_email_sent=false` (no se envía
correo al confirmar). No inicia sesión ni devuelve access/refresh.

Solo se instala el hash del intento confirmado. Se consume el intento y se
invalidan los demás del mismo correo en la transacción de activación. Después
de confirmar, otro registro o código nunca puede sustituir esa contraseña.

Código incorrecto o con formato inválido para un intento existente consume un
fallo persistido. Cinco fallos bloquean ese intento, incluso si después llega
el código correcto. Se mantienen los fallos después de reenvíos.

### Reenvío

`POST /api/v1/auth/resend-verification`, JSON con `registration_attempt_id`, sin correo.
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

### Límites, errores y clientes

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

### Enlaces y datos anteriores

GET `/api/v1/auth/verify-email` responde 410 informativo, ignorando query/token y sin
cambiar datos; HEAD 405. POST `{"token":...}` y reenvío `{"email":...}` ya no
son admitidos. Los enlaces anteriores no activan nada. La migración no purga
cuentas ni tokens reales. Una confirmación nueva válida puede reclamar una
cuenta antigua no verificada con su propio hash; cuentas verificadas por el
flujo anterior requieren auditoría aparte, sin corrección automática.

Antes de habilitar login: revisión técnica y de enumeración, auditoría de
cuentas heredadas, aprovisionamiento/rotación del secreto en todas las réplicas,
comunicación del cambio incompatible y pruebas de clientes en entorno aislado.
El cambio y la recuperación de contraseñas tienen un
[contrato separado](#contraseñas) e implementación con activación explícita;
no sustituyen la auditoría de cuentas heredadas.

## Sesiones

Las operaciones se definen en [auth-sessions.yaml](../openapi/auth-sessions.yaml).
El login permanece deshabilitado por defecto. La implementación local no acredita
la revisión operativa ni autoriza habilitación productiva.

### Login

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

### Refresh

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

### Restauración y logout

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

## Contraseñas

Contrato 0.1.0 con diseño aprobado el 2026-11-03 e implementación local con
activación explícita. Los valores de duración y cuotas requieren medición antes
del despliegue productivo. Las rutas exigen `AUTH_PASSWORDS_ENABLED=true` y un
secreto independiente; los esquemas se definen en
[auth-passwords.yaml](../openapi/auth-passwords.yaml).

### Reglas comunes

- HTTPS en despliegues reales; JSON UTF-8 y cuerpo máximo de 4096 bytes.
- Campos desconocidos, tipos incorrectos, UUID inválidos, Content-Type no admitido
  y JSON mal formado: 400 `/problems/malformed-request`. Cuerpo excesivo: 413.
- Respuestas, incluidos errores: `Cache-Control: no-store`, `Pragma: no-cache`,
  `X-Content-Type-Options: nosniff`. Errores: `application/problem+json`, con
  `type`, `title`, `status`, `detail`, sin datos internos ni secretos.
- Correo normalizado con trim y minúsculas, usando el validador de registro.
  Contraseñas sin trim ni normalización. Nueva contraseña: política vigente de
  registro, de 8 a 128 caracteres, mayúscula, minúscula, dígito ASCII y especial.
  Una contraseña nueva inválida devuelve 400 `/problems/invalid-password`.
- El código es un string de ocho dígitos; se preservan ceros iniciales. No es un
  JWT. Se envía únicamente por correo, nunca en respuesta HTTP ni URL.
- Contraseñas, códigos, tokens y cuerpos sensibles no aparecen en logs o métricas.
- Un 204 no tiene cuerpo y nunca entrega tokens. Después del cambio se requiere
  login. Las sesiones desktop y mobile se revocan con motivo `password_changed`.
- El JWT firmado puede seguir siendo aceptado por consumidores que no consulten
  el estado de sesión. Este contrato no promete revocación inmediata en Core/Files.

### 1. Cambio con sesión iniciada

`POST /api/v1/auth/change-password`

```http
Authorization: Bearer <access-token>
Content-Type: application/json
```

```json
{
  "current_password": "CurrentPass123!",
  "new_password": "NewPassword456!"
}
```

Auth verifica firma Ed25519, algoritmo permitido, `typ`, `kid`, emisor, audiencia
de Auth, vencimiento y vínculo `sub`/`sid`. Consulta la sesión autoritativamente
en PostgreSQL, sin llamar a su propia API HTTPS ni aceptar una credencial de
Core/Files. La cuenta debe estar activa, verificada y tener contraseña local.

La contraseña actual es obligatoria y se comprueba con Argon2id. No se aplica a
ella una política nueva de composición, solo el límite de 128 caracteres. La
nueva debe ser distinta de la actual. El cliente puede pedir repetirla en pantalla,
pero solo envía `new_password` al servidor.

| HTTP | Resultado |
|---|---|
| 204 | Contraseña actualizada; todas las sesiones revocadas |
| 400 | `/problems/invalid-current-password`, `/problems/invalid-password` o entrada inválida |
| 401 | `/problems/invalid-access-token`: token o sesión no utilizables; incluye `WWW-Authenticate: Bearer` |
| 409 | `/problems/password-change-not-allowed`: cuenta sin contraseña local |
| 409 | `/problems/password-unchanged`: nueva contraseña igual a la actual |
| 409 | `/problems/credentials-changed`: la credencial cambió durante la operación; no se sobrescribe |
| 429 | `/problems/too-many-requests`, con `Retry-After` entero positivo |
| 503 | `/problems/auth-unavailable`: dependencia, protección o trabajador no disponible antes del commit |
| 500 | `/problems/internal-error`, incluido commit de resultado incierto |

Una contraseña actual incorrecta no invalida por sí sola la sesión. Los clientes
no deben tratar este 400 como un logout ni borrar automáticamente el refresh.

### 2. Solicitud de recuperación

`POST /api/v1/auth/forgot-password`, sin autenticación:

```json
{"email": "person@example.invalid"}
```

Para cualquier correo de formato válido admitido por la protección de IP:

```http
HTTP/1.1 202 Accepted
```

```json
{
  "recovery_attempt_id": "a928e4c5-7ba2-4c64-9dc1-6f8c67fa7d9f",
  "expires_at": "2026-11-03T18:10:00Z",
  "message": "Si la cuenta permite recuperar la contraseña, recibirás un código por correo."
}
```

Se genera un UUID v4 nuevo y un vencimiento a diez minutos tanto para cuentas
elegibles como para correos desconocidos, cuentas bloqueadas/no verificadas o
cuentas sin contraseña local. No se devuelven `user_id`, `email_exists` ni
`verification_email_sent`. Los intentos sintéticos no permiten completar reset.

Solo una cuenta activa, verificada y con contraseña local recibe correo. Este
flujo no registra usuarios, verifica correos, desbloquea cuentas ni agrega una
contraseña a una identidad exclusivamente Google.

Solo se permite **un intento utilizable por cuenta**. Al admitir y persistir un
nuevo intento real, se invalida el anterior en la misma transacción. Su código
deja de funcionar de inmediato, aunque todavía no haya vencido. Se marca como
invalidado, no se borra físicamente hasta la limpieza prevista. El cliente conserva
el ID de su última solicitud y no combina códigos entre intentos.

La sustitución se serializa bloqueando la cuenta: dos solicitudes concurrentes
no pueden dejar dos intentos utilizables. Prevalece la última sustitución
confirmada en PostgreSQL, no necesariamente el último correo recibido. El nuevo
intento tiene su propio plazo de diez minutos y cinco fallos; las cuotas por IP
y destinatario siguen limitando la creación de reemplazos.

No hay endpoint separado de reenvío en esta propuesta. “Solicitar otro código”
crea otro intento, invalida el anterior y sustituye el ID mostrado por el cliente
solo si la solicitud se admite como intento real. Se propone cooldown
de envío por destinatario de 60 segundos y presupuesto compartido de correo con
registro/reenvío de capacidad 10 y recarga completa en 600 segundos.

Una supresión por cooldown o presupuesto del destinatario
mantiene el mismo 202 genérico con ID sintético. No expone 429 por cuenta. El
cliente no conoce si hubo envío y muestra siempre el mensaje genérico. Una
solicitud suprimida no crea un intento real ni invalida el vigente. La interfaz
debe espaciar las solicitudes al menos 60 segundos y advertir que se utilice el
código de la última solicitud admitida; el plazo local no garantiza admisión
porque otro cliente puede haber consumido el presupuesto. Esta ambigüedad se
conserva para no revelar la existencia de la cuenta o el estado del envío.

El correo se envía después de persistir el intento. Rechazo/timeout del proveedor
mantiene 202 sin revelar si la cuenta existe; el intento conserva su vencimiento.
El código anterior sigue invalidado aunque falle el envío del nuevo. No se
reactiva ni se recupera como fallback. Un rollback antes de confirmar la
sustitución conserva el intento anterior; un commit incierto no permite asegurar
cuál está vigente y se comunica como 500.
No hay reintento automático de envío en esta propuesta. Un fallo de PostgreSQL o
Redis necesario para admitir la solicitud produce 503 de manera independiente de
la existencia del correo: la admisión no omite dependencias para desconocidos.
No se promete igualdad exacta de latencia; la implementación deberá evitar enviar
correo solo en una rama observable sin mitigación y medir diferencias temporales.
La implementación local desacopla el envío en tareas en memoria: hasta 32 reservas
por proceso, mantenidas once segundos para solicitudes reales y sintéticas; envío
acotado a diez segundos y revisión de vigencia antes de enviarlo. No hay cola
durable: una caída puede perder el correo después del 202. Saturación devuelve
503 antes de buscar la cuenta. Esta estrategia reduce diferencias por proveedor,
pero requiere medición temporal antes de producción.

Esta política permite al usuario reemplazar un intento solicitado antes por otra
persona, pero también permite que otra persona solicite un reemplazo posterior
y deje inutilizable el código del usuario, dentro de las cuotas. Solicitar un
código no da acceso a la cuenta: también se necesita leer el correo y poseer el
ID correspondiente. Se acepta esta limitación de disponibilidad con la política
de último intento; no se promete un intervalo exclusivo para el propietario.

Además del 202: 400 por correo/entrada inválidos, 413 por tamaño, 429 por IP,
503 por indisponibilidad y 500 por fallo interno o commit incierto.

### 3. Confirmación de recuperación

`POST /api/v1/auth/reset-password`, sin autenticación:

```json
{
  "recovery_attempt_id": "a928e4c5-7ba2-4c64-9dc1-6f8c67fa7d9f",
  "code": "01234567",
  "new_password": "NewPassword456!"
}
```

Un código válido autoriza **solo** el reemplazo de contraseña para la cuenta e
intento vinculados, no una sesión ni la selección de otra cuenta. Se exige cuenta
activa, verificada y con contraseña local al confirmar, no solo al solicitar.

Duración absoluta: diez minutos desde creación. Máximo cinco fallos de código
persistidos por intento; el quinto lo bloquea. Para un ID UUID válido existente,
un código string con formato incorrecto también consume un fallo, sin importar
que `new_password` sea inválida. Campos ausentes, tipos incorrectos o UUID inválidos
se rechazan antes y no consumen un fallo identificable. Un código correcto con
contraseña nueva inválida o igual a la actual no se consume ni reinicia fallos.

La comprobación del código precede a los errores específicos de nueva contraseña
para no convertir esos errores en un oráculo sin prueba de posesión. Esto se
expresa también en OpenAPI: `new_password` se deserializa como string y su política
se comprueba después. La longitud total del cuerpo sigue acotada.

| HTTP | Resultado |
|---|---|
| 204 | Contraseña reemplazada, intento consumido y sesiones revocadas |
| 400 | `/problems/invalid-recovery-code`: ID desconocido/sintético, código incorrecto, vencido, bloqueado, consumido, invalidado o cuenta no elegible; mensaje indistinguible |
| 400 | `/problems/invalid-password`: solo después de comprobar un código válido |
| 409 | `/problems/password-unchanged`: código válido y contraseña igual a la actual |
| 409 | `/problems/credentials-changed`: carrera detectada; no sobrescribe la credencial |
| 429 | `/problems/too-many-requests`, por IP; `Retry-After` positivo |
| 503 | `/problems/auth-unavailable`, sin confirmar ningún cambio |
| 500 | `/problems/internal-error`, incluido resultado incierto del commit |

Un segundo uso después del éxito devuelve 400 genérico. No hay login automático.
Ante respuesta incierta, no repetir reset automáticamente: probar login con la
nueva contraseña; si no funciona, iniciar recuperación nueva. Un 503 explícito
no revoca sesiones ni obliga por sí solo a borrar refresh.

### Garantías de consistencia

Un cambio de credenciales posterior a la solicitud invalida su recuperación; un
intento no puede restaurar una contraseña anterior. El cambio, el consumo del
intento y la revocación de sesiones se confirman como conjunto. Los fallos de
código persisten incluso cuando la operación responde con error HTTP. Un fallo
anterior al commit conserva el estado previo; un resultado incierto no acredita
que la contraseña siga sin cambios.

La intención `session.revoked` con motivo `password_changed` se persiste junto a
la revocación. Eso no acredita publicación RabbitMQ ni integración de consumidores.
Los códigos no se almacenan en texto plano. La selección criptográfica, el orden
de bloqueos y la retención operativa se mantienen en la
[arquitectura de Auth](../../services/auth/docs/architecture.md#procesamiento-de-contraseñas)
y en [operación](../../services/auth/docs/operations.md#correo-y-retención).

### Presupuestos

Token buckets distribuidos, no ventanas fijas; recarga completa en el intervalo.
Separar operaciones públicas y privadas; la protección debe aplicarse también en
Nginx sin confiar en IP arbitraria enviada por el cliente.

| Operación | Clave | Capacidad / recarga completa |
|---|---|---|
| Change | IP | 6 / 60 s |
| Change | Cuenta autenticada | 5 / 900 s |
| Forgot | IP | 6 / 60 s |
| Reset | IP | 20 / 60 s |
| Envíos forgot | Destinatario, compartido con registro/reenvío | 10 / 600 s, cooldown 60 s |

Redis caído: las tres operaciones fallan con 503; no se hereda la excepción de
disponibilidad de la verificación de registro. Códigos mantienen además sus cinco
fallos en PostgreSQL. El presupuesto de destinatario es opaco; el de IP puede
devolver 429 antes de buscar cuenta. Las cuotas no garantizan disponibilidad
ante un atacante que ocupe sostenidamente los cupos.

### Selección del correo

Ver [contenido y plantilla](#correo-de-recuperación). El plazo del correo
debe provenir del mismo vencimiento absoluto que la API, nunca reiniciarse al enviar.
Idioma conforme a selección confiable del cliente/configuración, con fallback
es-MX; no se incorpora un campo nuevo de idioma al contrato HTTP.

### Criterios de aceptación previos a habilitar los flujos

1. JSON, errores y headers coherentes con OpenAPI; sin secretos en respuestas/logs.
2. Cuenta elegible recibe código; desconocida/bloqueada/no verificada/Google-only
   recibe respuesta indistinguible, sin creación o vinculación de cuentas.
3. Código ligado a intento, propósito y versión; vencimiento, cinco fallos,
   concurrencia, consumo único y supresión de envíos verificados. Un reemplazo
   confirmado invalida el anterior; una solicitud suprimida o un rollback no lo
   invalida. Fallo de envío posterior al commit no reactiva el anterior. Se verifica
   que dos sustituciones concurrentes solo dejen un intento utilizable.
4. Change requiere sesión y contraseña actual; rechaza refresh o token de servicio
   como access, y sesión revocada o perteneciente a otra cuenta.
5. Carreras reset/reset, change/reset, login/reset y refresh/reset no reactivan
   sesiones ni permiten instalar hashes calculados desde snapshots obsoletos.
6. Rollback previo a commit conserva el estado; commit incierto devuelve 500,
   sin afirmar que no hubo cambio. Fallos de código sí persisten al responder 400.
7. Mailpit real, mocks de clientes y Postman; pruebas de SQL y Redis complementarias.
8. Revisión de anti-enumeración temporal/envío, cuotas y parámetros; sin sustituir
   la auditoría pendiente de cuentas heredadas ni habilitar login por este documento.

## Correo de recuperación

El remitente usa el nombre Sonimbus y la dirección configurada por operación;
no contiene una dirección real incrustada en el contrato. El formato es multipart,
con texto plano y HTML equivalentes. Los textos aprobados se conservan a continuación.

### Español (es-MX, predeterminado)

**Asunto:** Recupera tu contraseña de Sonimbus

```text
Hola:

Recibimos una solicitud para restablecer la contraseña de tu cuenta de Sonimbus.

Tu código de recuperación es:

{{code}}

Introduce este código en la aplicación donde solicitaste la recuperación y elige
una nueva contraseña.

El código vence en 10 minutos y solo puede usarse una vez.

No compartas este código. El equipo de Sonimbus nunca te lo pedirá por correo,
teléfono o chat.

Si no solicitaste este cambio, ignora este mensaje. Esta solicitud por sí sola
no cambia tu contraseña ni cierra tus sesiones.

Cuando completes el cambio, se cerrarán tus sesiones y tendrás que iniciar
sesión con la nueva contraseña.

Equipo Sonimbus
```

### English (en / en-US)

**Subject:** Reset your Sonimbus password

```text
Hello,

We received a request to reset your Sonimbus account password.

Your recovery code is:

{{code}}

Enter this code in the application where you requested recovery and choose
a new password.

The code expires in 10 minutes and can only be used once.

Do not share this code. The Sonimbus team will never ask for it by email,
phone, or chat.

If you did not request this change, ignore this message. This request alone
does not change your password or sign you out.

After you complete the change, your sessions will be signed out and you
will need to sign in with your new password.

The Sonimbus team
```

### Reglas de renderizado y envío

- `code`: exactamente ocho dígitos como texto, incluidos ceros iniciales.
- El envío tardío no crea diez minutos nuevos. Si el intento ya venció o fue
  invalidado antes del envío, no iniciar el envío; un correo en vuelo no puede
  reactivar un intento invalidado.
- La recepción de un correo no prueba que sea el intento vigente: los mensajes
  pueden llegar fuera de orden. El cliente utiliza el ID de su propia solicitud;
  un reemplazo confirmado invalida códigos anteriores aunque su correo llegue después.
- HTML con el mismo contenido, código destacado y escape de todas las variables.
  Sin enlaces de activación, recursos externos, píxeles de seguimiento ni adjuntos.
- No incluir contraseña, JWT, refresh, ID de cuenta ni ID de intento. El cliente
  conserva el ID del intento; el correo no sustituye esa asociación.
- No incluir el código en el asunto. No registrar cuerpo, código ni MAC.
- Una solicitud de recuperación no debe revocar sesiones. La revocación ocurre
  únicamente después de completar el cambio de contraseña correctamente.
