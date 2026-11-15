# Arquitectura de Auth

Esta guía describe la implementación. Las rutas, cuerpos y errores públicos se
definen en [contratos](../../../contracts/auth/public-api.md#alcance-y-pendientes).

## Responsabilidades

| Módulo | Responsabilidad |
|---|---|
| `handlers` | Extracción HTTP, autenticación requerida por la operación y traducción de errores |
| `dto` y `validators` | Estructura y validación de entradas y salidas |
| `services` | Reglas de negocio y coordinación de transacciones |
| `repositories` | SQL, persistencia y bloqueos de datos |
| `security` | Argon2id, códigos HMAC y tokens Ed25519 |
| `protection` e `infrastructure` | Identidad de red, presupuestos y conexión Redis |
| `ports`, `providers` y `email` | Contratos de envío, transportes y contenido de correo |
| `internal` | Listener HTTPS privado, credenciales y admisión de consultas |
| `config`, `main` y `state` | Configuración, composición de dependencias y estado compartido |

`lib.rs` expone los componentes de tokens y consulta de sesiones sin iniciar el
servidor. `src/tests` accede a módulos internos mediante `cfg(test)`; `tests`
comprueba la interfaz de biblioteca y las restricciones de PostgreSQL.

## Registro y verificación

[Contrato de registro](../../../contracts/auth/public-api.md#registro-y-verificación) y
[ADR-008](../../../docs/decisions/ADR-008-registration-attempt-codes.md).

El registro crea un intento independiente en `auth.registration_attempts`, con
hash de contraseña y MAC de código ligados al intento. La confirmación instala
únicamente la contraseña de ese intento, verifica la cuenta, consume el intento
e invalida los demás en una transacción. Se crea una cuenta nueva sin reclamar
cuentas con propuestas pendientes. Los contactos canónicos existentes, incluidos
los de cuentas bloqueadas, no se sobrescriben. La implementación no sanea automáticamente cuentas heredadas.

La confirmación serializa por correo, bloquea el intento y crea su propia cuenta. La elegibilidad se revalida bajo
bloqueo. Los fallos de código se confirman antes de devolver el error HTTP; un
fallo SQL revierte la activación completa. Las cuentas heredadas y sus sesiones
no se transfieren al nuevo propietario del contacto.

El envío ocurre después del commit, sin retener bloqueos durante la llamada al
proveedor. Un fallo conserva el intento y se comunica mediante
`verification_email_sent`. Un reenvío reemplaza la MAC del mismo intento sin
prolongar su vencimiento ni reiniciar fallos. No existe una cola durable de envío.

## Procesamiento de contraseñas

`PasswordProcessor` comparte entre registro, login y contraseñas un presupuesto
de trabajo Argon2id. El cálculo se ejecuta en `spawn_blocking`; el permiso se
conserva hasta que termina el trabajo, incluso si se cancela la solicitud.

`PasswordService` coordina cambio y recuperación con acceso a datos mediante
`passwords_repository`. La configuración del secreto independiente se describe
en [configuración](configuration.md#secretos). Las reglas observables pertenecen
al [contrato de contraseñas](../../../contracts/auth/public-api.md#contraseñas).

El trabajo costoso usa una instantánea de credenciales. Antes de escribir se
revalida su versión bajo bloqueo. El cambio de contraseña, el consumo o
invalidación de intentos y la revocación de sesiones se confirman juntos. Las
carreras con login, refresh y otros cambios se serializan mediante la cuenta;
ninguna ruta debe adquirir esos bloqueos en orden inverso.

La recuperación usa admisión local acotada para solicitudes reales y sintéticas.
El proveedor se invoca después del commit en una tarea en memoria, sin esperar
su resultado en HTTP. Un cierre del proceso puede perder un envío ya aceptado;
no hay reintento automático ni persistencia de códigos en texto plano. Una tarea
horaria elimina intentos terminales o expirados tras la retención de 24 horas y
se detiene cuando termina el servidor. Los mensajes en vuelo pueden llegar fuera
de orden y nunca reactivan un intento invalidado.

## Sesiones y transacciones

[Flujos de sesión](../../../contracts/auth/public-api.md#sesiones) y
[perfil de tokens](../../../contracts/auth/tokens.md).

PostgreSQL conserva sesiones y hashes de refresh. El orden común es cuenta,
credencial, sesión y refresh; `auth.open_session` añade bloqueo advisory por
cuenta y tipo de cliente. La búsqueda inicial de un hash descubre identidad,
pero no autoriza. Todas las comprobaciones relevantes se repiten bajo bloqueo.

Login verifica Argon2id fuera de la transacción, revalida hash y versión y firma
el access antes de confirmar. Una cuenta inexistente también realiza trabajo de
hash para reducir diferencias evidentes, sin prometer tiempos idénticos.

Refresh guarda el hash del sucesor y marca el anterior consumido en la misma
transacción. El replay revoca la familia y confirma esa revocación antes del
error. Logout revoca únicamente la sesión identificada por el refresh. Un
resultado incierto del commit produce 500; no se presenta como un 503 que
asegure que la credencial no se consumió. No hay reintento automático.

Las migraciones se agregan sin reescribir versiones aplicadas:

| Migración | Propósito |
|---|---|
| `0001` | Esquema inicial de Auth |
| `0002` | Intención transaccional de revocación y trigger de refresh |
| `0003` | Bloqueos, duración e inmutabilidad de sesión |
| `0004` | Duración absoluta en segundos, independiente de cambios DST |
| `0005` | Intentos de registro ligados a códigos |
| `0006` | Recuperación de contraseñas |
| `0007` | Expansión de propuestas y códigos de contacto, sin trasladar datos existentes |
| `0008` | Procedencia y evidencia de sesión, con sesiones anteriores `legacy` |
| `0009` | Intentos federados y consumo durable |
| `0010` | Eventos de vinculación y outbox de avisos a contacto verificado |
| `0011` | Transición de contacto, elegibilidad por método y apertura común de sesiones |
| `0012` | Reserva única durable del callback antes del intercambio externo |
| `0013` | Índice de avisos pendientes que conservan presupuesto de entrega |

La restricción de vida absoluta incorporada como `NOT VALID` protege nuevas
escrituras; validar datos anteriores requiere auditoría. La retención de hashes
de refresh debe conservar la detección de replay durante la vida de la sesión.
La limpieza de recuperación no elimina cuentas, sesiones ni esos hashes.

## Consulta autoritativa de sesiones

El [contrato interno](../../../contracts/auth/internal-api.md) define actividad,
orden y cardinalidad. `SessionCheckService` valida el lote antes de consultar.
El repositorio usa arrays parametrizados y `unnest ... WITH ORDINALITY` para
resolver todos los pares mediante una sentencia, sin consultas por elemento.

La consulta usa `statement_timestamp()` y una instantánea MVCC conjunta de cuenta
y sesión. No abre bloqueos de escritura, actualiza actividad ni genera eventos.
El pool apunta al primario de Auth, sin réplica retrasada, caché ni fallback.
Los resultados incompletos o mal asociados fallan como conjunto; un fallo de
infraestructura nunca se convierte en una sesión inactiva fabricada.

El resultado describe el instante observado y no reserva autorización. La
verificación JWT y los permisos de grupos o archivos pertenecen al consumidor.
Los consumidores no acceden directamente al esquema `auth`.

## Evolución aprobada: Google y correo de contacto

El [contrato técnico v1.0](../../../contracts/auth/public-api.md#google-y-correo-de-contacto)
centraliza rutas, identidad, contacto, vinculación, errores y condiciones de
aceptación. El esquema, la elegibilidad por método y el adaptador OIDC están implementados; los
endpoints de login Google están implementados; vinculación y verificación de
contacto siguen pendientes.

La evolución separará autenticación del contacto, permitirá cuentas Google sin
correo principal acreditado y reutilizará la apertura transaccional de sesiones.
La confirmación de registro, la de contacto y la asignación automática competirán
por una sola exclusividad de dirección verificada. Los contactos pendientes no
reservarán correos y nunca aportarán credenciales de otra cuenta.

El [diseño de migraciones](#diseño-de-migraciones-google-y-contacto) concreta la
transición de datos, las restricciones y los cambios coordinados de consultas.
La migración `0011` y sus cambios coordinados separan contacto de elegibilidad
de sesión, conservando el rechazo de cuentas locales pendientes. Las migraciones
`0007`–`0010` incorporan la expansión aditiva. Ninguna de estas etapas habilita
por sí misma los endpoints Google.
Las migraciones `0001`–`0006` no se reescriben.

Las responsabilidades se mantendrán: handlers HTTP, un servicio de coordinación
Google, un puerto/adaptador OIDC, repositorios SQL y un recorrido común de sesión.
No se ampliará el servicio de registro con todas las responsabilidades del proveedor.
La criptografía, expiración y envío podrán compartirse entre verificaciones,
con propósitos separados para registro y contacto.

### Adaptador OIDC Google disponible

`ports/federated_identity.rs` define el puerto de autorización e intercambio y el
resultado de identidad. `providers/google` implementa configuración, transporte,
validación de claims y caché de claves. La biblioteca los expone como
`auth_service::federated_identity` y `auth_service::google_oidc`, siguiendo el
patrón utilizado para tokens y consulta de sesiones.

El adaptador usa los endpoints HTTPS fijos publicados en el
[discovery de Google](https://accounts.google.com/.well-known/openid-configuration),
sin seguir redirecciones ni aceptar ubicaciones de claves del token. Solo solicita
`openid email`. El intercambio tiene presupuesto total de 8 segundos, conexión de
2 segundos y solicitud HTTP de 5 segundos. Los cuerpos se limitan durante lectura
a 32 KiB para tokens y 64 KiB para JWKS; el ID token admite hasta 16 KiB.

La validación acepta exclusivamente RS256 y comprueba firma, emisor, audiencia,
`azp`, expiración, `iat`, `nbf` cuando existe, subject y el hash SHA-256 del `nonce`.
La comparación del nonce es constante. El emisor se normaliza al identificador
HTTPS de Google después de verificar las dos formas admitidas por el proveedor.
Los campos de correo se evalúan según las
[reglas de autoridad de Google](https://developers.google.com/identity/gsi/web/guides/verify-google-id-token):
un correo externo sin Workspace queda pendiente aunque Google informe
`email_verified=true`. Esa prueba nunca evita la exclusividad de contacto SQL.

La caché admite hasta 16 claves RSA de 2048–4096 bits, respeta `Cache-Control` y
`Age` con máximo local de una hora y limita las descargas a una cada 30 segundos
por instancia. La actualización se serializa, también ante `kid` desconocido.
Las claves vencidas no autorizan identidades durante un fallo de actualización.
Los errores públicos son categorías finitas y no contienen respuestas del
proveedor, códigos ni credenciales. No se persisten tokens Google.

### Coordinación del login Google

`GoogleAuthService` coordina el puerto OIDC y `FederatedAuthRepository`; los
handlers solo extraen HTTP y traducen respuestas. El servicio se incorpora a
`AppState` con activación explícita. Las operaciones implementadas se describen
en [OpenAPI Google](../../../contracts/openapi/auth-google.yaml). No se requiere
login local habilitado para crear una sesión Google.

La creación entrega un secreto de intento y un ticket independiente de navegador.
State y nonce se derivan del ticket aleatorio con SHA-256 y etiquetas de propósito
separadas; únicamente sus hashes llegan a PostgreSQL. El ticket de authorize se
consume condicionalmente y la cookie por intento usa prefijo `__Host-`, Path=/,
Secure, HttpOnly y SameSite=Lax. HEAD no consume tickets ni callbacks. Los parámetros
HTTP duplicados y las cookies ambiguas se rechazan.

`0012_federated_callback_claims.sql` reserva cada callback mediante una fila única
antes del canje externo, sin mantener transacciones ni conexiones SQL durante
Google. Una caída posterior a la reserva obliga a iniciar otro flujo. Callback
valida nonce, registra prueba mínima y entrega un código de 60 segundos al destino
registrado, sin crear todavía cuentas o sesiones.

Exchange comprueba secreto, código y PKCE S256. Serializa asignación de correo e
identidad externa y bloquea la cuenta antes de abrir sesión. Recupera cuentas
por identidad Google; una dirección externa no las vincula por coincidencia de
correo. Si el contacto acreditado ya pertenece a otra cuenta, devuelve
`account-link-required`, sin cambios parciales. Las cuentas bloqueadas no reciben
sesión. No se altera el contacto de una identidad ya vinculada.

La cuenta nueva, identidad, propuesta o contacto, sesión, hash de refresh y consumo
del intento se confirman en una sola transacción. Se reutilizan la apertura SQL
y emisión de tokens de sesiones; un fallo revierte todos esos efectos. Un commit
incierto responde 500 y nunca se reintenta automáticamente. Las pruebas y reservas
se eliminan en lotes de hasta 1000, después de 24 horas de expiración, mediante una
tarea horaria que se detiene con el proceso.

Las rutas comparten un presupuesto Redis Google por IP usando `AUTH_GOOGLE_IP`
(20 solicitudes y recarga completa en 60 segundos), con clave separada del login local y rechazo cerrado si Redis falla.
El gateway agrega un presupuesto Google de 60 solicitudes/minuto y burst 12 por IP.
El callback admite hasta 16 llamadas simultáneas por proceso y timeout OIDC de
8 segundos; Nginx espera hasta 15 segundos para esa ruta. Estos valores iniciales
requieren medición antes de atribuirles capacidad productiva.

### Vinculación y reautenticación

`GoogleAuthService` comparte creación, navegador, callback y pruebas de exchange
entre login y vinculación. El propósito procede de PostgreSQL; el cliente no elige
cuenta ni sesión. `SessionAuthenticationRepository` bloquea cuenta, credencial y
sesión en ese orden, y comprueba la evidencia del servidor. `GoogleConnectionsRepository`
resuelve exclusividad de identidad y proveedor, registra el evento y la outbox y
consume el intento en la misma transacción. Los advisory locks de correo e identidad
se adquieren antes de los bloqueos de cuenta para conservar el orden del login.

La sesión iniciadora debe seguir activa y conservar exactamente la fecha, método
y versión capturados al crear el intento. El consumo vuelve a comprobar la vigencia
de sesión y los diez minutos de evidencia. Logout, bloqueo, cambio de contraseña,
expiración o nueva reautenticación exigen iniciar otro intento. Una vinculación
correcta o repetida responde 204, sin cambiar contacto, contraseña, JWT ni refresh.
Los conflictos no fusionan cuentas ni sustituyen identidades.

`SessionService::reauthenticate` usa `PasswordProcessor` fuera de transacciones,
captura la fecha efectiva y revalida sesión y credenciales bajo bloqueo. Actualiza
solo la evidencia mutable; conserva la procedencia de apertura. Redis limita por
IP y por cuenta a seis solicitudes con recarga de 60 segundos, entre réplicas.
Las rutas autenticadas siguen sujetas a `AUTH_GOOGLE_ENABLED`.

El evento se conserva aun sin contacto verificado. La notificación solo se encola
para un contacto acreditado y se deduplica por evento. El worker de entrega está
implementado con activación independiente. La presentación de eventos en la interfaz
queda sujeta a la integración de clientes.

### Verificación general de contacto

`ContactEmailService` coordina códigos y envío mediante `ContactEmailRepository`.
Las rutas se definen en [OpenAPI contacto](../../../contracts/openapi/auth-contact-email.yaml).
La autenticación HTTP de Google y contacto comparte `handlers/authentication`, con
validación de JWT y consulta autoritativa; los repositorios revalidan bajo bloqueo.
La función no depende de Google ni de la existencia de contraseña local.

`VerificationCodeKey` concentra generación CSPRNG y MAC constante; registro conserva
exactamente su formato anterior, comprobado con un vector de compatibilidad.
`ContactEmailCodeKey` añade propósito propio y contexto de cuenta, dirección, intento,
versión de propuesta y versión de código. Recuperación reutiliza la generación
común y conserva su MAC existente. Los códigos permanecen como texto y solo su MAC
se guarda en PostgreSQL.

La propuesta se modifica bajo bloqueo de cuenta. Repetir una dirección equivalente
no renueva la versión ni invalida pruebas. Se evita un UPSERT de propuestas porque
su trigger BEFORE INSERT incrementaría la versión incluso ante conflicto.
Las lecturas previas evitan gastar cuotas en reintentos idempotentes o intentos
no utilizables; no autorizan escrituras. La confirmación toma primero el advisory de correo compartido con registro y Google,
y después cuenta, credencial, sesión e intento. Asignación y consumo son atómicos
bajo `accounts_email_unique`. La retirada de la propuesta sucede después de consumir
el intento, conservando el registro terminal y la versión monotónica.

Los fallos de código se confirman antes del error HTTP. El quinto bloquea el intento;
reenviar no amplía plazo ni reinicia fallos. El envío ocurre después del commit,
con un máximo local de 16 operaciones y presupuesto de 12 segundos por llamada.
Un error conserva el intento reintentable y no convierte 202 en garantía de entrega.
Las plantillas de registro, contacto, recuperación y aviso son distintas y usan
el puerto de correo y el contenido común `EmailContent`.

### Entrega de avisos de seguridad

`SecurityNotificationService` procesa la outbox mediante `SecurityNotificationsRepository`.
Cada reserva autocommit usa `FOR UPDATE SKIP LOCKED`, incrementa el contador y ocupa
la fila durante 60 segundos. El contador funciona como versión: una confirmación
atrasada no puede actualizar una reserva posterior. El envío y su timeout de 12 segundos
suceden fuera de transacciones y sin retener conexiones SQL.

El worker procesa hasta 20 filas por pasada, con intervalo de 5 segundos y un solo
envío simultáneo por proceso. El presupuesto por fila es de diez reservas, con
backoff desde 30 segundos hasta una hora. `0013` indexa únicamente pendientes con
presupuesto restante. Solo se procesan eventos Google conocidos y contactos que
siguen verificados al reservar; no se usa correo del proveedor como alternativa.
Los errores persistidos son categorías finitas, sin respuestas SMTP/HTTP ni destinatarios
en logs. Las filas agotadas y sus eventos se conservan para revisión operativa.

La aceptación por el proveedor y la confirmación SQL no constituyen una transacción
externa única. Un reinicio en esa ventana puede repetir el aviso. La deduplicación
del evento/outbox evita generar otra notificación por una vinculación idempotente,
pero no promete una sola entrega SMTP. Las tareas periódicas y el worker se cancelan
al terminar el servidor; las reservas sin confirmar vuelven a ser elegibles al vencer.
La limpieza horaria de contraseñas, Google y contacto comparte un coordinador de
mantenimiento y elimina intentos antiguos en lotes acotados.

## Diseño de migraciones Google y contacto

**Estado: migraciones `0007`–`0013`, login, vinculación, contacto y worker de avisos implementados con activación explícita.** El contrato
funcional aprobado permanece en [Google y contacto v1.0](../../../contracts/auth/public-api.md#google-y-correo-de-contacto).
Esta sección distingue el soporte de datos disponible de los flujos HTTP pendientes. Las pruebas aplican las migraciones a bases desechables;
no acreditan aplicación a la base de desarrollo ni a un despliegue compartido.

### Esquema actual y decisión de modelado

`accounts.email` es `CITEXT UNIQUE` nullable y contiene exclusivamente un contacto
verificado, emparejado con `email_verified_at`. `oauth_identities` limita
identidad/proveedor y cuenta/proveedor. Registro crea una cuenta nueva al confirmar;
no modifica cuentas heredadas que solo tienen una propuesta pendiente. Login local
exige contacto verificado y contraseña; refresh y consulta privada aplican la
elegibilidad específica del método de apertura.

El modelo objetivo conserva **`accounts.email` como único contacto verificado**,
pero permite que sea nulo. Se conserva `accounts_email_unique`: PostgreSQL admite
varias filas con valor nulo bajo esa restricción y sigue rechazando dos contactos
no nulos iguales según CITEXT. No se sustituye por tres índices independientes ni
por un índice parcial que deje de satisfacer `ON CONFLICT (email)`.

La pareja `email`/`email_verified_at` cumple una restricción de integridad:
ambos son nulos o ambos tienen valor. La presencia del correo principal significa
que existe acreditación; una dirección informada por un proveedor nunca se escribe
allí sin esa prueba. El estado administrativo sigue siendo independiente.

Las direcciones propuestas se almacenan fuera de `accounts.email`, sin unicidad
por dirección. Puede haber varias propuestas del mismo correo en cuentas distintas,
pero únicamente un propietario confirmado. Los intentos de registro permanecen
independientes y tampoco reservan el correo.

### Contacto pendiente y códigos

| Estructura objetivo | Datos principales y restricciones |
|---|---|
| `accounts` | Contacto opcional, exclusividad existente y contador monotónico `contact_email_version` |
| `account_email_proposals` | Una propuesta por cuenta: `account_id`, `email`, versión y fechas; FK a cuenta, sin índice único por correo |
| `contact_email_verification_attempts` | UUID, cuenta, correo/versiones de instantánea, MAC de 32 bytes, versión de código, fallos, fechas de creación/envío/expiración y estados terminales |

La versión del contacto reside en la cuenta y no se reinicia al eliminar una
propuesta. Reemplazar una dirección incrementa esa versión e invalida intentos
previos; volver a proponer la misma dirección no hace utilizable un código antiguo.
El contador no puede decrecer ni desbordarse silenciosamente. El trigger de
propuestas incrementa la versión al insertar, sustituir o eliminar una dirección,
e invalida las pruebas pendientes. Repetir una propuesta equivalente no renueva
su versión. Los escritores bloquean primero la cuenta antes de modificar su
propuesta o sus intentos.

Un intento admite entre cero y cinco fallos y entre cero y tres reenvíos. El
vencimiento es posterior a creación y no excede diez minutos. Se conservan marcas
de consumo e invalidación coherentes y un máximo de un intento pendiente por
cuenta. El índice de pendientes depende de estados terminales, no de `now()`:
antes de crear otro intento se invalida el anterior, incluso si ya expiró.

La FK acredita la existencia de la cuenta; la autorización de la sesión y la
coincidencia de cuenta, propuesta, correo y versión se revalidan bajo bloqueo.
El propósito criptográfico es distinto de registro y recuperación. El almacén
solo conserva la MAC; no conserva el código ni permite usar una comprobación de
contacto para instalar contraseñas.

Confirmar contacto bloquea la cuenta y su intento, comprueba la instantánea,
asigna `accounts.email` y `email_verified_at` y consume la prueba en una sola
transacción. La violación `23505` de `accounts_email_unique` produce el conflicto
de contacto; no se traducen indiscriminadamente todos los errores SQL al mismo
resultado. Confirmar registro y crear Google con contacto acreditado escriben en
esa misma columna y chocan con esa misma restricción.

### Método de sesión y autenticación reciente

La evidencia de método se asocia a la sesión mediante campos nuevos en
`auth.sessions`, sin cambiar el payload de JWT ni el contrato de consulta interna.

| Dato lógico | Regla |
|---|---|
| `authentication_method` | `legacy`, `password` o `google`; método de apertura fijado por el servidor |
| `oauth_identity_id` | Solo para Google, asociado a la misma cuenta de la sesión |
| `password_credential_version` | Instantánea de contraseña cuando el método es local |
| `last_authenticated_at` y método de evidencia reciente | Solo se registran tras una comprobación efectiva; pueden ser nulos si no existe evidencia suficiente |

Las sesiones preexistentes reciben `legacy` y no adquieren autenticación reciente
por el hecho de migrarse. No se deduce una comprobación reciente de `created_at`,
`last_seen_at`, `last_login_at`, `iat` o refresh. Un login local nuevo registra el
instante real de verificación; una actualización posterior de presencia o refresh
no lo modifica. La prueba Google de identidad no equivale automáticamente a una
reautenticación interactiva reciente si el proveedor reutilizó su sesión.

La relación sesión/identidad debe garantizar también la cuenta correspondiente,
mediante FK compuesta a una clave única de identidad/cuenta. Se agregará esa clave
sin retirar las restricciones `(provider, provider_subject)` y `(account_id, provider)`.
Los campos concretos de reautenticación son `last_authenticated_at`,
`reauthentication_method`, `reauthentication_password_version` y
`reauthentication_oauth_identity_id`. Las FK vinculan identidad, cuenta y proveedor,
impidiendo usar una identidad de otro proveedor como Google. Los CHECK distinguen
combinaciones válidas de método, identidad y credencial;
los campos de procedencia de apertura quedan protegidos frente a cambios junto
a la identidad inmutable de sesión. La evidencia de reautenticación sí se puede
actualizar por su operación autorizada y no después de revocación o vencimiento.

`auth.open_authenticated_session` concentra la apertura transaccional y recibe
un contexto tipado de autenticación. El login local registra método, versión de
contraseña y fecha de comprobación efectiva. `auth.open_session` conserva su firma
como envoltorio compatible para sesiones `legacy` sin evidencia reciente. Refresh
no modifica esa evidencia ni el método de apertura.

La elegibilidad común conserva cuenta activa, pertenencia, no revocación y
vencimiento. Para `google` exige identidad Google asociada a esa cuenta y prueba
de apertura válida; no exige contacto. Para `password` y `legacy` exige contacto
verificado y credencial local utilizable. No basta con que una cuenta tenga
alguna identidad Google: una sesión local pendiente no puede heredar esa excepción.

`SessionsRepository` separa el bloqueo de cuenta activa del requisito de
contraseña local. Refresh, operaciones de contraseña y consulta interna comparten
`auth.session_account_status` para evaluar la elegibilidad de la sesión concreta. El método de apertura no desaparece al rotar refresh.
Retirar una identidad en una futura funcionalidad deberá invalidar sus sesiones;
no se habilita una desvinculación mediante esta migración.

### Intentos federados y consumo durable

`federated_auth_attempts` conserva un registro durable por intento, con propósito
`login` o `link`. Su modelo incluye:

- UUID, proveedor fijo, cliente, destino registrado y fechas de creación/expiración.
- Hash del secreto iniciador, ticket de navegador, `state` y `nonce`; desafío S256.
- Cuenta y sesión iniciadoras para `link`, con versión de credencial y evidencia
  de reautenticación que deben seguir siendo válidas al finalizar.
- Resultado mínimo validado por el backend: emisor/subject, correo informado,
  autoridad sobre ese correo y límite de validez de la prueba de identidad.
- Hash del código de finalización, vencimiento y estado terminal.
- Asociación del resultado con cuenta, sesión creada o evento de vinculación,
  sin almacenar un par de access/refresh recuperable.

No se guardan código OAuth, verificador PKCE, ID token completo, access ni refresh
Google. Los secretos opacos se almacenan como hashes de valores de alta entropía,
no como contraseñas humanas. Los valores públicos y los claims mínimos se tratan
como datos personales, sin volcarlos en logs.

Los estados distinguen intento creado, autorización en curso, resultado Google
validado, consumido, cancelado, fallido y expirado. Las transiciones se validan
mediante actualizaciones condicionales y restricciones de fechas/campos; un intento
terminal no vuelve a ser utilizable. El código de finalización dura como máximo
60 segundos y nunca supera la expiración del intento o de la prueba de identidad.

Los hashes tienen longitud fija y los valores que permiten localizar tickets,
`state` y códigos de finalización tienen unicidad cuando están presentes. Los
CHECK de propósito exigen cuenta y sesión iniciadoras para `link`, pero no permiten
que un intento de login seleccione una cuenta existente por un identificador del
cliente. Una FK compuesta garantiza que la sesión iniciadora pertenezca a esa
cuenta; exige una clave única adicional `(id, account_id)` en sesiones. Las fechas
y el resultado requerido se vinculan al estado: marcar consumo sin su resultado
confirmado no es una transición válida.

El callback adquiere el derecho a procesar una vez, termina su transacción y
contacta a Google. Después persiste el resultado validado en otra transacción.
Ninguna llamada externa mantiene bloqueos SQL. Una caída durante el canje deja
un intento fallido/no finalizable, sin repetir automáticamente el código Google.
Redis puede acelerar estados efímeros, pero no decide si hubo consumo definitivo.

Exchange bloquea y revalida el intento, cuenta y sesión según el orden común,
crea los cambios necesarios y marca consumo en la misma transacción. La reserva
previa de un intento nunca sustituye esa revalidación. Solo una finalización
puede confirmar y no se reemiten tokens al repetir una solicitud ya consumida.
El resultado incierto de commit conserva el tratamiento conservador de 500.

### Eventos y avisos de seguridad

`account_security_events` registra la nueva vinculación con identificador estable,
cuenta, identidad, fecha y estado de lectura en la aplicación. La unicidad por identidad y tipo
evita que una vinculación idempotente produzca avisos duplicados. La outbox tiene como máximo una fila por
evento y canal; el estado de entrega no autoriza volver a vincular. No se usa
`session_revocation_outbox` para este fin: esa tabla representa revocaciones y conserva su contrato actual.

`security_notification_outbox` relaciona un evento con su aviso de correo cuando
existe contacto verificado. El destinatario es una instantánea del contacto
acreditado en el momento del evento; nunca se toma del correo externo informado
por Google. La selección y persistencia del aviso pertenecen a la misma transacción
de vinculación. El trabajador envía después del commit y registra resultado sin
revertir la asociación. No se promete entrega exactamente una vez si el proveedor
acepta el correo y se pierde su respuesta.

Sin contacto acreditado se conserva el evento para la aplicación, sin crear un
correo alternativo. Las consultas de eventos pertenecen únicamente a su cuenta.
Los plazos de retención y el procesamiento de avisos se cerrarán con la política
operativa antes del despliegue; no se copia automáticamente la retención de códigos
ni se presenta esta tabla como integración con notificaciones Core o RabbitMQ.

### Secuencia de migraciones y despliegue

`0007`–`0010` incorporan la expansión compatible; `0011` incluye la transición
del contacto junto con repositorios, DTO y consultas compatibles. SQLx aplica
esta secuencia al arrancar; debe desplegarse como una unidad coordinada.

| Etapa | Migración | Contenido y compatibilidad |
|---|---|---|
| Expansión de contacto | `0007_contact_email_support.sql` | Tablas de propuestas/intentos y versión; conserva columnas y restricciones actuales durante la convivencia |
| Evidencia de sesión | `0008_session_authentication_evidence.sql` | Procedencia, FK de identidad y checks; sesiones antiguas `legacy`, sin fabricar prueba reciente |
| Coordinación federada | `0009_federated_authentication_attempts.sql` | Estados, hashes, resultado mínimo y consumo final durable |
| Avisos | `0010_account_security_events.sql` | Eventos de vinculación y outbox específica, todavía sin habilitar Google |
| Transición de contacto | `0011_verified_contact_cutover.sql` | Contacto opcional, traslado de pendientes, consistencia de correo/fecha, elegibilidad común y apertura con contexto tipado |

Durante la expansión `0007`–`0010` se conserva `accounts.email NOT NULL`,
`accounts_email_unique`, los datos y la firma de `auth.open_session`. Ninguna sesión
anterior recibe prueba reciente por defecto. La ampliación del trigger de
inmutabilidad protege también la procedencia de las sesiones nuevas.

La expansión no habilita Google. La transición `0011` requiere que todos los
procesos usen el código incluido compatible con contacto opcional,
propuestas separadas y la nueva elegibilidad. SQLx aplica las migraciones pendientes
al iniciar Auth: un archivo de cutover no debe añadirse como cambio aislado antes
de preparar repositorios, DTO y consultas públicas/privadas correspondientes.

Antes del cutover se detienen escritores incompatibles, incluidos procesos de
otras réplicas y tareas que accedan al esquema. Se verifica el respaldo y la
posibilidad de restauración. El inventario previo distingue cuentas verificadas,
pendientes, bloqueadas, identidades externas preexistentes, sesiones y tokens
heredados. Una dirección vacía, normalización ambigua, credencial huérfana o
inconsistencia de cuenta/identidad impide continuar hasta su revisión. No se
resuelven anomalías marcando correos como verificados, fusionando cuentas o
eliminando filas silenciosamente.

Para datos consistentes, los contactos verificados se conservan con su timestamp;
los no verificados pasan a propuestas no exclusivas y sus columnas principales
quedan nulas. Se conserva cada `account_id`, estado administrativo y credencial
existente. Preservar un hash heredado no lo convierte en método utilizable ni
permite instalarlo en otra cuenta. Los códigos heredados no se reclasifican como
prueba de contacto. Las sesiones `legacy` de cuentas sin contacto acreditado
siguen sin ser autorizables; no reciben procedencia Google por inferencia.

La dirección pendiente de una cuenta bloqueada no se convierte en una reserva
permanente. El bloqueo sigue asociado a esa cuenta e identidad; no se interpreta
el correo no acreditado como identidad de toda persona que lo use. Esta diferencia
frente a la búsqueda anterior por correo se incluye expresamente en el inventario
y la revisión de datos heredados antes de activar la transición.

El cutover comprueba que todos los correos principales cumplen la nueva pareja
de campos y conserva `accounts_email_unique` como última barrera de concurrencia.
No se depende únicamente del advisory lock actual de registro. El orden global
de bloqueos será común a registro, contacto y federación: advisory de asignación
por correo normalizado, advisory de identidad externa cuando corresponda, cuentas
en orden estable, credenciales, sesiones en orden estable e intentos. Refresh
conserva cuenta → sesión → token. Ningún recorrido mantiene un intento bloqueado
mientras adquiere después cuenta o sesión en orden inverso. Las lecturas de
identidad previas a ese orden no autorizan; cualquier reserva previa se libera
antes de adquirir bloqueos en orden contrario. Se prueban interbloqueos y
conflictos entre flujos antes de habilitar réplicas.

Después de admitir cuentas con contacto nulo no es seguro volver al binario
anterior ni reponer `NOT NULL` automáticamente. El rollback es operativo: detener
la nueva función, evaluar restauración íntegra del respaldo o realizar una
corrección hacia delante. No se rellenan contactos con direcciones ficticias
para satisfacer restricciones antiguas.

### Código y comprobaciones asociados al cutover

Los cambios coordinados incluyen `Account.email` y sus DTO de lectura, consultas
de disponibilidad, confirmación de registro, búsqueda local por contacto,
elegibilidad de recuperación, sesiones, refresh y consulta interna. La nueva
operación de contacto no modifica por sí sola las credenciales. Los endpoints
de registro actuales conservan sus respuestas de éxito con correo acreditado;
los contratos de lectura que incorporen contacto opcional lo representan como nulo.

La validación usa PostgreSQL desechable desde un esquema vacío y fixtures
previos a la transición. Las pruebas de esquema cubren las restricciones; los
casos de endpoints federados deberán completarse al implementar sus flujos:

1. Preservación de cuentas, timestamps verificados, hashes, estados y revocaciones;
   ningún dato pendiente se acredita ni se fusiona automáticamente.
2. Varias propuestas con igual dirección y varias cuentas sin contacto, pero un
   solo propietario verificado por CITEXT; ningún código previo sobrevive a un
   cambio de versión o sirve para otro propósito.
3. Carreras reales entre registro, contacto y Google sobre la misma dirección:
   un commit ganador, error `23505` identificable para el otro y cero efectos parciales.
4. Sesiones Google sin contacto admitidas por refresh y consulta interna; sesiones
   locales pendientes, identidades ajenas y cuentas bloqueadas rechazadas.
5. Refresh no actualiza evidencia reciente; migrar o cambiar presencia tampoco.
6. Dos exchanges concurrentes, repetición tras commit y caída de Redis no producen
   dos sesiones ni dos vinculaciones; el consumo durable se revierte con el resto
   de cambios cuando falla la transacción antes de confirmar.
7. Un evento y como máximo un aviso pendiente por vinculación nueva; el correo
   del proveedor no se selecciona como fallback y un fallo de entrega no deshace el vínculo.
8. Compatibilidad de fixtures y de toda la suite Auth, restricciones de sesión,
   outbox de revocación y validación de los OpenAPI actualizados.

## Protección y límites de confianza

El router privado nunca se fusiona con el público. Su listener valida CIDR desde
el socket, TLS, cuotas previas, credencial, permiso y reserva antes de consultar.
Las IP IPv4 mapeadas en IPv6 se normalizan; las cabeceras de proxy no establecen
identidad interna. El pool privado es de solo lectura y tiene timeout SQL.

Redis coordina presupuestos y reservas; no almacena la autoridad de sesión.
Los scripts Lua usan el reloj de Redis y ejecutan admisión atómica. Un timeout
no demuestra que el consumo no ocurrió, por lo que no se repite automáticamente.
Las reservas distribuidas dependen de la conservación del estado de Redis;
una pérdida de datos puede reiniciar cuotas y no equivale a cancelación del
trabajo ya iniciado. Los detalles operativos se mantienen en [operación](operations.md).

La outbox conserva `session.revoked` en la transacción de revocación. No existe
aún publicación RabbitMQ ni consumo distribuido. Persistir el evento no garantiza
desconexión inmediata de Core, Files o WebSocket.
