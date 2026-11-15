# API pública de Auth

Esta guía reúne los acuerdos de registro, sesiones, contraseñas y autenticación federada para móvil,
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

La [especificación Google y correo de contacto](#google-y-correo-de-contacto) es
el diseño aprobado v1.0. El esquema y la transición de contacto están implementados;
el login Google y su OpenAPI están implementados; vinculación y contacto siguen
pendientes.
Los archivos OpenAPI actuales describen las operaciones ya incorporadas; esta
sección no acredita endpoints Google disponibles ni modifica su comportamiento.

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
Un contacto canónico existente, incluso en cuenta bloqueada, mantiene 409; esto conserva enumeración conocida.

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
cuentas ni tokens reales. Una confirmación nueva válida crea su propia cuenta y
credencial; no reclama cuentas heredadas ni sus sesiones. Las direcciones antiguas
no verificadas pasan a propuestas sin exclusividad. Las cuentas verificadas por
el flujo anterior requieren auditoría aparte, sin corrección automática.

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

## Google y correo de contacto

**Versión 1.0. Diseño aprobado el 2026-11-09; login implementado con activación explícita.** Esta
sección es la fuente normativa de CU-04, la vinculación de Google y CU-02 Verificar
correo de contacto. Sustituye las propuestas de verificación obligatoria después
de Google, contraseña obligatoria en cada vinculación y vinculación automática
por coincidencia de correo. El [OpenAPI Google](../openapi/auth-google.yaml)
describe login, vinculación explícita, conexiones y reautenticación por contraseña.
La [verificación general de contacto](../openapi/auth-contact-email.yaml) y el worker
de avisos están implementados con banderas independientes. No existe aprobación de despliegue.

El [diseño de migraciones](../../services/auth/docs/architecture.md#diseño-de-migraciones-google-y-contacto)
concreta el modelo y la transición segura. Las migraciones `0007`–`0013`, el
contacto opcional y la elegibilidad de sesión por método están implementados.
El adaptador OIDC y el login Google ya están disponibles con activación explícita;
la vinculación, reautenticación por contraseña, verificación de contacto y avisos
están incorporados con activación explícita. El cambio no acredita direcciones
antiguas ni habilita acceso mediante Google.

### Alcance y conceptos

Google acredita una identidad mediante OpenID Connect; Auth emite las sesiones,
access y refresh de Sonimbus con el perfil existente. El inicio de sesión y el
registro Google no exigen un código de correo de Sonimbus. Google decide sus
propias comprobaciones, incluidas passkeys o coincidencia de números; Sonimbus
no exige ni reproduce un método concreto de segundo factor de Google.

| Concepto | Significado y autoridad |
|---|---|
| Cuenta | Identificador interno de Sonimbus y estado administrativo |
| Identidad Google | Emisor validado y `sub` estable, representados como proveedor Google y su subject |
| Correo del proveedor | Atributo informado por Google; no cambia por sí solo el contacto de Sonimbus |
| Contacto pendiente | Dirección propuesta, sin propiedad acreditada ni reserva exclusiva |
| Contacto verificado | Dirección acreditada y asociada de forma exclusiva a una cuenta |
| Autenticación reciente | Evidencia del servidor de una comprobación efectiva de credenciales, independiente del refresh |

La identidad externa, el estado administrativo y la verificación de contacto
son condiciones distintas. Una cuenta activa autenticada mediante Google puede
carecer de contacto verificado. No se genera una contraseña artificial. El
correo principal es opcional en el modelo objetivo y no identifica la cuenta.

No forman parte de esta versión la fusión de cuentas, sustitución o desvinculación
de Google, sincronización automática de perfil, acceso a Drive/Gmail, cambio de
un contacto ya verificado ni recuperación asistida por soporte. Añadir contraseña
local requiere un flujo separado: verificar contacto no lo ejecuta implícitamente.

### Exclusividad y resolución de cuentas

La identidad Google se busca por emisor y `sub`, nunca únicamente por correo.
Las variantes de emisor admitidas de Google se normalizan a un único proveedor
confiable, sin permitir proveedores o URL de claves enviados por el cliente.

| Situación | Resultado |
|---|---|
| Identidad ya vinculada a cuenta activa | Inicia sesión, aunque no tenga contacto verificado |
| Cuenta vinculada bloqueada, suspendida o eliminada | Acceso denegado; no se crea otra cuenta |
| Identidad nueva y correo Gmail/Workspace acreditado y disponible | Crea cuenta e identidad, asigna contacto verificado y abre sesión en una transacción |
| Identidad nueva con dirección externa sobre la que Google no es autoridad | Crea cuenta e identidad y abre sesión; la dirección queda como propuesta no exclusiva |
| Correo acreditado por Google ya verificado en otra cuenta | Devuelve `account-link-required`; exige entrar en esa cuenta antes de conectar Google |
| Dirección externa informada coincide con contacto de otra cuenta | No autoriza asociación ni recuperación de esa cuenta; la nueva identidad solo puede tener contacto pendiente |
| Correo de una identidad ya vinculada cambia en Google | Conserva la vinculación por `sub`; no modifica el contacto de Sonimbus |

Para aceptar automáticamente un contacto se exige `email_verified=true` y que
Google sea autoridad sobre él: Gmail, o Workspace con `hd` validado. Una dirección
externa puede aparecer verificada por Google sin acreditar su propiedad actual;
por ello permanece pendiente en Sonimbus. Los atributos ausentes o no acreditados
no autorizan contacto automático. Un token válido con correo no acreditado no
obliga a introducir un código para iniciar sesión.

La confirmación de registro con contraseña, la confirmación de contacto y la
asignación automática de Gmail/Workspace compiten por el mismo recurso exclusivo
y la misma restricción única en PostgreSQL. La asignación y los cambios de cuenta,
identidad o intento se confirman juntos. Una comprobación previa no sustituye
esa restricción. Gana la transacción que confirma la asignación; la otra devuelve
conflicto y revierte sus cambios parciales. Para la misma cuenta y dirección,
una confirmación repetida puede responder de forma idempotente.

Las direcciones pendientes no reservan ni bloquean el registro de su propietario.
La confirmación nunca copia una contraseña o una identidad desde otra cuenta.
Una cuenta heredada pendiente no se activa automáticamente mediante Google ni
aporta sus credenciales a una cuenta nueva; su revisión se realizará en el diseño
de migraciones, conservando el estado administrativo y evitando apropiaciones.

### Contrato HTTP y recorrido Google

Las rutas `google/attempts`, `google/authorize`, `google/callback` y
`google/exchange` implementan login. `google/link-attempts`, `reauthenticate` y
`connections` también están implementadas. Los endpoints de contacto tienen
su propio OpenAPI y activación independiente. La integración permanece deshabilitada por defecto hasta su aprovisionamiento:
cliente OAuth confidencial del backend, callback HTTPS fijo, audiencias, destinos
de retorno permitidos y claves de propósito. Ningún secreto confidencial se
distribuye a las aplicaciones. Los valores reales pertenecen al entorno protegido,
no al contrato ni a sus ejemplos. La activación conserva las revisiones operativas
existentes; habilitar Google no habilita automáticamente login por contraseña.
Las solicitudes JSON rechazan campos desconocidos y usan nombres en inglés.
`client_type` conserva `desktop` o `mobile`; `device_name` es opcional, no nulo
cuando aparece y de hasta 120 caracteres. Los cuerpos tienen máximo 4096 bytes.

| Método y ruta bajo `/api/v1/auth` | Autorización | Entrada y resultado |
|---|---|---|
| `POST /google/attempts` | Pública, con protección de solicitudes | `{client_type, device_name?, redirect_target, code_challenge}`; `201` con intento de login |
| `GET /google/authorize` | Ticket de navegador de un uso | Inicia correlación del navegador y responde `302` hacia Google |
| `GET /google/callback` | Respuesta Google correlacionada | Valida respuesta y redirige al destino autorizado con un código temporal |
| `POST /google/exchange` | Secreto del intento y prueba PKCE | `{attempt_id, completion_code, code_verifier}`; `200 TokenResponse` para login o `204` para vinculación |
| `POST /google/link-attempts` | Access Sonimbus, sesión activa y autenticación reciente | `{redirect_target, code_challenge}`; `201` con intento ligado a cuenta y sesión |
| `POST /reauthenticate` | Access y sesión activa | `{password}` para una cuenta con contraseña local; `204`, actualiza evidencia del servidor sin crear sesión |
| `GET /connections` | Access y sesión activa | Lista el proveedor y correo informado de las conexiones propias, sin tokens, hashes ni subject externo |

La respuesta `201` de creación contiene `attempt_id` UUID, `attempt_secret`,
`authorization_url` y `expires_at` UTC RFC 3339. El secreto es aleatorio de al
menos 256 bits y solo se entrega por HTTPS. El navegador recibe un ticket de
lanzamiento distinto; nunca recibe el secreto del intento o el verificador PKCE.
`GET /connections` responde `{connections:[{provider:"google",provider_email,
linked_at}]}`; `provider_email` puede ser nulo. No expone conexiones de otra cuenta.

El cliente conserva en memoria un verificador aleatorio de al menos 256 bits.
`code_challenge` es base64url sin padding de SHA-256 del verificador; solo se admite
S256. En exchange envía el secreto mediante `X-Auth-Attempt-Secret`. La prueba PKCE
protege el código de finalización de Sonimbus y es distinta del código OAuth que
Google entrega exclusivamente al callback del backend.

`redirect_target` es un identificador registrado de cliente, no una URL arbitraria.
Cada destino configura esquemas, host y ruta admitidos. El loopback de escritorio,
si se utiliza, solo admite la dirección local y el puerto validado de ese intento;
móvil usa destinos de aplicación registrados y verificables. La activación de un
cliente requiere completar ese registro, sin comodines de redirección externos.

El recorrido es navegador del sistema → Auth authorize → Google → callback HTTPS
fijo de Auth → destino del cliente → exchange. El callback comprueba `state`,
correlación del navegador y `nonce`; rechaza parámetros de seguridad duplicados,
combinaciones inconsistentes y callbacks repetidos. Los parámetros informativos
adicionales de Google se ignoran sin reflejarlos. La cookie de correlación es
por intento, usa prefijo `__Host-`, Path=/, Secure, HttpOnly y SameSite=Lax; no es una sesión de usuario ni autoriza exchange.

El backend intercambia el código por el ID token, fuera de transacciones SQL,
y valida firma, algoritmo permitido, emisor, audiencia configurada, vencimiento,
`nonce` e identidad autorizada del cliente cuando corresponda. Las claves solo
proceden de configuración/discovery confiables de Google, con caché acotada y
actualización limitada ante `kid` desconocido. No se usa el verificador Ed25519
de Sonimbus para aceptar tokens Google. Solo se solicitan `openid email`; no se
persisten access ni refresh de Google ni el ID token completo.

El retorno implementado incluye `attempt_id` y `completion_code`; en cancelación
o fallo incluye `attempt_id` y `error=cancelled|failed`. HEAD responde 405 sin
consumir el intento. Una reserva durable evita canjes Google concurrentes del
mismo callback; una caída después de reservar requiere iniciar otro flujo.

El callback no crea sesión ni vinculación. Entrega un código de finalización de
un solo uso ligado a intento y desafío. El código no es un access ni un refresh.
Exchange verifica las pruebas, revalida cuenta, sesión y versión de credencial
cuando corresponda y realiza el cambio definitivo. En login crea cuenta/identidad
si es necesario, abre sesión mediante el recorrido común, genera refresh y firma
access antes de commit. Conserva duración, reemplazo por tipo de cliente y outbox
existentes. Una respuesta perdida no autoriza repetir automáticamente exchange.

Un intento cancelado o inválido no crea cuenta, sesión ni vinculación. El callback
solo redirige a un destino previamente validado; ante un estado no reconocible
presenta un error genérico sin usar destinos proporcionados por esa solicitud.
Los errores de Google no se reflejan literalmente. La aplicación puede recibir
un resultado finito de cancelación o fallo, nunca credenciales en la URL.

### Vinculación y autenticación reciente

La vinculación se inicia desde Configuración → Cuentas conectadas, con acción
explícita. El backend obtiene `account_id` y `sid` de la sesión autoritativa;
no acepta esos identificadores como selección de cuenta en el cuerpo.

Se permite conectar una cuenta Google cuyo correo difiera del contacto Sonimbus.
No modifica ese contacto, su verificación, la contraseña o las sesiones existentes,
ni emite un nuevo par de tokens. El usuario ve qué cuenta Google se va a conectar.

Una autenticación es reciente durante diez minutos desde una comprobación efectiva
de credenciales, según tiempo del servidor. Un login con contraseña puede aportar
esa evidencia; refresh, actividad y `iat` de un nuevo JWT no la renuevan. Si falta
o venció, la cuenta local utiliza reautenticación con contraseña antes de iniciar.
El backend limita intentos y liga la evidencia a cuenta, sesión, método y versión.
La reautenticación con un proveedor ya vinculado debe acreditar esa misma identidad;
un token Google silenciosamente renovado no demuestra por sí solo presencia reciente.
Su adaptación para operaciones futuras se diseñará antes de habilitarlas.

El intento de vinculación conserva propósito, cuenta, sesión y versión de
credencial. Al finalizar, la sesión debe seguir activa y la evidencia conservar
su vigencia. Logout, bloqueo, cambio de credenciales o vencimiento impiden completar
la operación; se exige un intento nuevo después de reautenticar.

| Conflicto | Resultado |
|---|---|
| La misma identidad ya está en esa cuenta | `204` idempotente, sin otro aviso de vinculación |
| La identidad está asociada a otra cuenta | `409 identity-already-linked` |
| La cuenta ya tiene otra identidad Google | `409 provider-already-linked`; no se sustituye |
| El correo Google coincide con contacto verificado de otra cuenta | `409 account-link-conflict`; no se fusionan cuentas |

La coincidencia entre correos no es requisito de vinculación y tampoco acredita
control de una cuenta distinta. La restricción de identidad se comprueba de nuevo
al confirmar la transacción para resolver carreras entre intentos.

La implementación registra el evento y encola el aviso al contacto verificado
junto con el vínculo. No usa como destinatario el correo informado por Google
cuando falta ese contacto. El worker requiere `AUTH_SECURITY_NOTIFICATIONS_ENABLED`
y reintenta de forma acotada, sin mantener transacciones durante SMTP/HTTP.
Una fila en outbox no acredita entrega; `delivered_at` registra aceptación del
proveedor. Una caída entre envío y confirmación puede repetir el mismo aviso:
la entrega externa tiene semántica de al menos una vez, sin crear otro evento.
La presentación de eventos en la interfaz sigue pendiente de integración del cliente.

La reautenticación por contraseña admite seis solicitudes por IP y seis por
cuenta cada 60 segundos mediante presupuestos Redis independientes. Los límites
se comparten entre réplicas y los fallos de Redis rechazan la operación. En esta
etapa, las tres operaciones autenticadas requieren `AUTH_GOOGLE_ENABLED`.

### Verificar correo de contacto

CU-02 es general e independiente del proveedor. Su actor es un usuario autenticado
con cuenta activa y sin contacto verificado. Puede iniciarse desde el perfil o
al intentar una operación que necesite acreditar esa dirección. No se ejecuta
obligatoriamente después de Google ni presenta avisos en cada inicio de sesión.

| Método y ruta bajo `/api/v1/auth` | Entrada y resultado |
|---|---|
| `GET /contact-email` | `200 {contact_email, contact_email_verified, pending_contact_email}`; las direcciones ausentes son nulas |
| `POST /contact-email/verification-attempts` | `{email}`; `202 {verification_attempt_id, expires_at}`; `204` si ya es el contacto verificado de esa cuenta |
| `POST /contact-email/verification-resends` | `{verification_attempt_id}`; `202` con el mismo identificador y vencimiento |
| `POST /contact-email/verify` | `{verification_attempt_id, code}`; `204` al asignar el contacto |

Todas requieren access y sesión activa. El correo se normaliza con la política
actual, sin reglas específicas que eliminen puntos o sufijos de Gmail. La
solicitud no acepta un `account_id` de destino. Una dirección distinta sustituye
la propuesta pendiente e invalida sus códigos. Un contacto ya verificado para la
misma cuenta no requiere otro código; la confirmación es idempotente. Cambiar un
contacto ya verificado queda fuera del alcance y devuelve conflicto.

Las cuatro operaciones requieren `AUTH_CONTACT_EMAIL_ENABLED`, independiente de
Google. Repetir la misma propuesta vigente conserva identificador y vencimiento,
sin generar otro código ni reenviar correo. Un reintento de confirmación con el
código consumido válido sobre ese mismo contacto responde 204 durante la retención.
La verificación no renueva evidencia de autenticación reciente ni crea contraseñas
o sesiones. Las solicitudes conocidas de un registro que pierde la exclusividad
frente a contacto o Google responden conflicto, conservando sus garantías de consumo.

El código tiene ocho dígitos CSPRNG y se trata como texto. La vigencia es de diez
minutos; el quinto fallo bloquea el intento. El reenvío requiere 60 segundos,
admite como máximo tres sustituciones y no amplía vencimiento ni reinicia fallos.
La MAC liga propósito `contact-email-verification`, cuenta, dirección normalizada,
identificador y versión. La confirmación exige la misma cuenta autenticada y
asigna la dirección bajo la restricción común de exclusividad.

Un código con formato incorrecto cuenta como fallo cuando el intento válido está
identificado y autorizado; un cuerpo mal formado no revela ni modifica intentos
de otras cuentas. Los fallos persistidos no se revierten al devolver el error.
El envío sucede fuera de la transacción. `202` acredita aceptación de la solicitud,
no entrega del correo. Un fallo de proveedor conserva un estado reintentable,
sin activar contacto. Las políticas de abuso comparten el presupuesto de envío
por destinatario e incorporan límites por cuenta e IP; no dependen solo del código.

Redis usa presupuestos de IP separados: lectura (capacidad 30), envío (6) y
confirmación (20), con recarga completa de 60 segundos. Por cuenta, envío y
confirmación tienen capacidades 6 y 20, con recarga de 600 segundos; crear y
reenviar comparten envío. El presupuesto por destinatario se comparte con registro.
Una reserva de presupuesto puede consumirse aunque una carrera impida el envío.
Los fallos de Redis rechazan la operación. Estos valores iniciales requieren medición.

Registro con contraseña y verificación de contacto reutilizan el mecanismo de
códigos, MAC, expiración, consumo, envío y fallos, pero mantienen contextos y
propósitos diferentes. El visitante confirma el registro ligado a su intento y
contraseña; el usuario autenticado verifica contacto sin cambiar credenciales.
CU-01 no incluye un CU cuya precondición exija una sesión inexistente. La
verificación de contacto no activa cuentas heredadas ni añade identidades.

### Operaciones que requieren contacto verificado

La lista inicial es cerrada; una ampliación exige cambio explícito del contrato.

| Operación | Condición adicional |
|---|---|
| Reclamar una invitación dirigida a una dirección | El contacto verificado debe coincidir con la dirección invitada |
| Añadir contraseña local | Contacto verificado y autenticación reciente con el método existente; flujo separado |
| Recuperar contraseña por correo | Contacto verificado y contraseña local previamente existente |
| Elegir destinatario de avisos de seguridad por correo | Debe ser un contacto verificado de esa cuenta |

No se exige contacto verificado para iniciar o renovar una sesión Google, cerrar
sesión, consultar la cuenta, conectar Google con las pruebas exigidas ni solicitar
verificación. Tampoco se añade como requisito global a grupos, mensajes, voz,
archivos o proyectos: esos recursos conservan sus permisos y reglas de dominio.
Los consumidores no deducen requisitos adicionales del correo informado por Google.

La consulta interna de sesiones debe aceptar una sesión obtenida mediante Google
validado aunque no exista contacto verificado. El requisito de verificación del
registro con contraseña se conserva; no se elimina globalmente para admitir
cuentas locales pendientes. La evidencia del método de autenticación es del
servidor. Los resultados internos mantienen sus campos actuales y no convierten
el estado de contacto en una autorización de recursos.

### Notificaciones y recuperación

La interfaz distingue «Google conectado» de «Correo de contacto». Puede mostrar
el correo informado por Google como referencia, con el contacto pendiente marcado
como no verificado, sin afirmar que la identidad Google esté sin autenticar.
La lectura `GET /contact-email` no expone direcciones de otra cuenta.

Después del commit de una vinculación nueva se registra un evento de seguridad
y se presenta aviso en la aplicación. Si existe contacto verificado, se envía
además un correo informativo, sin código de confirmación. No se usa como fallback
una dirección externa informada por Google. El fallo del envío no revierte una
vinculación ya confirmada; los eventos necesarios para seguimiento se conservan
sin incluir secretos. La persistencia y entrega de esos avisos se diseñarán con
las migraciones, sin afirmar que exista actualmente una infraestructura de avisos.

Una cuenta exclusivamente Google sin otro método de acceso depende de Google
para recuperarse. Verificar contacto no crea una contraseña ni habilita por sí
solo recuperación local. Perder Google antes de configurar otro método puede
impedir recuperar el acceso a Sonimbus; soporte no promete una restitución fuera
de un procedimiento aprobado. El perfil informa esta dependencia y permite
iniciar voluntariamente la verificación, sin bloquear el uso ordinario.

### Límites, estados y errores

| Control | Valor o regla |
|---|---|
| Intento Google | Diez minutos |
| Autenticación reciente para vincular | Diez minutos, revalidada al finalizar |
| Código de finalización | 60 segundos y nunca posterior al vencimiento del intento |
| Ticket de navegador y callback | Uso único y ligado al intento |
| Finalización correcta | Un solo commit; consume durablemente la autorización |
| Dependencias externas | Fuera de transacciones; sin reintento ciego del canje de códigos |

Redis puede conservar estado efímero, pero PostgreSQL debe coordinar el consumo
final durable con la creación de cuenta, identidad o sesión. Borrar una clave
Redis antes o después del commit no basta para garantizar consumo único. Una
caída o pérdida de estado no permite reconstruir una autorización favorable ni
fabricar una finalización. Los límites concretos de Nginx, intentos, conexiones,
llamadas Google y tareas de correo se definirán antes de habilitar el despliegue,
sin relajar expiración, consumo único o consistencia transaccional.

Los errores HTTP usan `application/problem+json` con `type`, `title`, `status`
y `detail` genérico, sin incluir tokens, direcciones ajenas ni diagnósticos del
proveedor. Las respuestas sensibles llevan `Cache-Control: no-store`,
`Pragma: no-cache` y `X-Content-Type-Options: nosniff`.

| HTTP | `type` bajo `/problems/` | Aplicación |
|---|---|---|
| 400 | `malformed-request` | JSON, tipos o campos no admitidos |
| 400 | `invalid-verification-code` | Código incorrecto, bloqueado, vencido o intento no utilizable |
| 401 | `invalid-google-credentials` | Prueba Google, intento, secreto o prueba PKCE inválidos |
| 401 | `invalid-access-token` | Sesión Sonimbus ausente, inválida o revocada; añade Bearer challenge |
| 401 | `invalid-credentials` | Contraseña incorrecta al reautenticar; sin revelar datos de la credencial |
| 401 | `reauthentication-required` | Falta evidencia reciente para vincular o finalizar |
| 403 | `account-not-allowed` | Cuenta autenticada no habilitada; sin revelar detalles administrativos |
| 409 | `account-link-required` | Contacto Google acreditado ya pertenece a una cuenta existente |
| 409 | `identity-already-linked` / `provider-already-linked` / `account-link-conflict` | Conflicto de vinculación |
| 409 | `contact-email-in-use` / `contact-email-change-not-supported` | Exclusividad de contacto o cambio fuera de alcance |
| 413 | `request-too-large` | Cuerpo mayor que el máximo |
| 429 | `too-many-requests` | Cuota agotada; incluye `Retry-After` positivo |
| 503 | `auth-unavailable` | Google, Redis, PostgreSQL u otra dependencia impide completar sin cambio confirmado |
| 500 | `internal-error` | Fallo interno o commit incierto; no afirma ausencia de cambios |

Las acciones autenticadas sobre contacto aceptan que un conflicto indique que la
dirección no está disponible, sin revelar su propietario. No existe una consulta
pública de cuentas por correo. Un cliente no borra refresh ni deduce revocación
por una indisponibilidad. Ante exchange o commit de resultado incierto inicia
un flujo nuevo, sin repetir automáticamente el código anterior.

### Condiciones de aceptación

1. Crear o recuperar la misma identidad Google no produce cuentas duplicadas;
   correos externos no bloquean direcciones mediante propuestas pendientes.
2. El ID token de otro cliente, emisor, nonce, algoritmo o clave no permite sesión;
   cancelar o repetir callback/exchange no crea cambios adicionales.
3. Una cuenta Google sin contacto verificado puede operar y renovar sesión; una
   cuenta local pendiente no obtiene acceso por esa excepción.
4. Registro, contacto y Gmail/Workspace concurrentes chocan con la misma
   exclusividad; no hay dos propietarios verificados ni efectos parciales.
5. Vincular correos distintos conserva el contacto original; no fusiona cuentas,
   reemplaza otra identidad ni acepta una sesión revocada durante el recorrido.
6. Refresh no renueva la autenticación reciente y las carreras con cambio de
   contraseña, logout o bloqueo se revalidan antes de confirmar.
7. Los códigos de contacto no sirven para registro, recuperación, login o enlace
   de identidades; cambiar la dirección pendiente invalida códigos anteriores.
8. Los correos informativos solo salen a contactos verificados tras el commit;
   su fallo no revierte la vinculación y no expone credenciales en logs.
9. La conservación y entrega de avisos, la caché de claves y los límites externos
   se validan en entorno aislado antes de habilitar Google.

Referencias técnicas: [Google OpenID Connect](https://developers.google.com/identity/openid-connect/openid-connect),
[validación e identidad de correo](https://developers.google.com/identity/gsi/web/guides/verify-google-id-token),
[vinculación segura OWASP](https://cheatsheetseries.owasp.org/cheatsheets/Authentication_Cheat_Sheet.html#secure-federated-account-linking)
y [navegador externo en aplicaciones nativas](https://www.rfc-editor.org/rfc/rfc8252.html).

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
