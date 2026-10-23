# Sesiones, login, refresh y logout: tercera entrega

Refs #35 y #17. Base `b5451ef`, rama de trabajo `feature/auth-sessions`.
La referencia `feature/auth-jwt-jwks` se conserva en `b5451ef`.
No se cambia de rama ni se integra/reorganiza el historial de los PR anteriores.
El PR #44 seguía abierto al inspeccionarlo; la API GitHub no devolvió un PR de
`feature/auth-jwt-jwks`. Esa ausencia no acredita integración de fase 2.

## Estado y bloqueo de habilitación

Se implementan repositorios, transacciones y POST `/api/v1/auth/login`,
`/api/v1/auth/refresh`, `/api/v1/auth/logout`. Se reutiliza el emisor Ed25519
de [fase 2](TOKENS.md), incluyendo una salida de duración calculada con el mismo
reloj de emisión. No se reconstruye firma, JWKS o gestión de claves.

**Login permanece deshabilitado por defecto**: `AUTH_LOGIN_ENABLED=false`.
Responde 503 `/problems/auth-unavailable`. Solo las pruebas o un entorno de
desarrollo aislado deben habilitarlo explícitamente mientras #17 siga abierto.
Refresh y logout mantienen su protección y disponibilidad independientes de esa
bandera. La implementación no constituye autorización para habilitar login en producción.

#17 sigue sin mitigación aprobada: alguien puede registrar un correo ajeno con
su contraseña y su dueño terminar verificándolo mediante un enlace. Comprobar
`email_verified_at` no reemplaza esa contraseña. Se necesita acordar un flujo que
entregue al dueño control exclusivo de credenciales, coherente con enumeración
y carreras de verificación. No se elige silenciosamente re-registro, purga o
recuperación de contraseña en esta entrega.

## Persistencia y orden de bloqueos

No se modifica `0001`. Las nuevas migraciones son:

- `0002_session_revocation_outbox.sql`: tabla mínima de intención JSON y trigger
  transaccional al pasar sesión de no revocada a revocada. También revoca sus
  refresh. Captura logout, replay y reemplazos, incluso invocando la función SQL.
  Un evento estable por sid; sin FK para no perder la intención al limpiar sesiones.
- `0003_session_lifetime_and_locking.sql`: reemplaza `auth.open_session`, valida
  TTL positivo de hasta 20 días, bloquea cuenta primero y usa reloj real tras
  esperar bloqueos. Fija tiempos a segundos UTC para no redondear tokens hacia
  arriba. Protege identidad, expiración absoluta y revocación frente a reactivación.
- `0004_session_elapsed_duration.sql`: expresa el límite como 1 728 000 segundos
  transcurridos, evitando que días de calendario agreguen una hora al cruzar DST.

El CHECK de vida absoluta se agrega NOT VALID: protege nuevas escrituras sin
validar masivamente datos anteriores. Antes de validar la restricción en un
despliegue con sesiones heredadas, auditar su duración. No se incluye limpieza.

Orden común: cuenta `FOR UPDATE`, credencial de contraseña para login, bloqueo
advisory de cuenta/tipo al abrir, sesión `FOR UPDATE`, refresh. La búsqueda inicial
por hash solo descubre identidad y no autoriza. Después se revalida todo bajo
bloqueos. El bloqueo de cuenta serializa también operaciones de distinto tipo;
sus sesiones siguen siendo independientes. No hay retry automático de transacciones.

El futuro cambio de contraseña/estado debe usar cuenta antes de credencial/sesión
y revocar sesiones en esa transacción; no adquirir bloqueos en orden inverso.
No se implementa aquí ese flujo. Las pruebas cubren cambios entre lectura inicial
y apertura; no se promete revocar automáticamente por cualquier UPDATE externo.

## Login

Normaliza correo con trim/lowercase y reutiliza su validador. La contraseña no se
recorta, normaliza ni somete a una nueva política de composición; se limita a los
128 caracteres ya admitidos en registro. `client_type` es desktop/mobile explícito;
`device_name` es opcional de hasta 120 caracteres. No se deduce por User-Agent.

Lee una instantánea de hash/versión, verifica Argon2 fuera del runtime mediante
`spawn_blocking`, compartiendo el semáforo existente con registro. Para una cuenta
inexistente realiza también una operación de hash. Esto reduce diferencias obvias
de trabajo; no garantiza igualdad temporal ni resuelve toda la enumeración.

Con credenciales válidas abre transacción, bloquea cuenta y credencial, comprueba
estado activo, correo verificado y que hash/versión sigan iguales. Abre/reemplaza
con `auth.open_session`, inserta primer refresh y firma access antes de commit.
Una firma o persistencia fallida revierte reemplazo, refresh y outbox juntos.
Solo devuelve credenciales después de commit. Estado no habilitado y credenciales
incorrectas comparten 401 `/problems/invalid-credentials`.

## Refresh y logout

Refresh es 32 bytes CSPRNG, base64url sin padding (43 caracteres); se conserva solo
SHA-256 de los bytes decodificados. Se utiliza el mismo vencimiento absoluto de
sesión para cada sucesor. No se elimina ningún hash histórico en esta entrega.
La retención y limpieza posterior deben conservar detección de replay durante
toda la vida de sesión; la falta de limpieza actual implica crecimiento de datos.

Con cuenta y sesión utilizables, bloquea refresh. Inserta sucesor y marca anterior
consumido con `replaced_by_token_id` y `revoked_at`. Firma antes de commit. Replay
de uno consumido revoca la familia con `refresh_reuse` y confirma esa transacción
**antes** de devolver 401 `/problems/invalid-refresh-token`.

Dos refresh simultáneos pueden producir un 200 y un 401 que revoca también el
sucesor del primero. No hay ventana de gracia ni recuperación del secreto sucesor.
El cliente debe coordinar renovaciones; si pierde respuesta, no repite el refresh.

Logout identifica familia mediante refresh, incluso consumido. Revoca únicamente
esa sesión; desconocido o ya revocado retorna 204. No acepta sid arbitrario como
credencial. Un logout concurrente con refresh no permite reactivar la sesión.
Los access emitidos requieren la futura comprobación de sesiones en consumidores;
esta entrega no garantiza desconexión ni revocación instantánea fuera de Auth.

Outbox contiene `session.revoked` v1 con los seis motivos del esquema. No publica
en RabbitMQ ni tiene worker, confirms, reintentos o consumidores. El flujo de
eventos sigue incompleto. La expiración se rechaza por reloj sin depender de un
evento; marcar expiradas y publicar sus eventos requiere trabajo posterior.

## HTTP y protección

JSON de hasta 4 KiB en rutas de sesión; entrada inválida, campos extra, tipos
incorrectos o JSON no utilizable devuelven 400 Problem Details sin reflejar cuerpo.
Refresh exige longitud de 1–512 antes de validar su contenido. Ningún DTO de
credenciales muestra sus valores en Debug. No se registran SQLx errors ni cuerpos
de estas operaciones. Respuestas tienen `Cache-Control: no-store`, `Pragma:
no-cache`, `nosniff`; 401 incluye Bearer challenge.

Errores: 400 malformed-request, 401 invalid-credentials/invalid-refresh-token,
429 too-many-requests con Retry-After, 503 auth-unavailable y 500 internal-error.
Un fallo al confirmar transacción se devuelve como 500 de resultado incierto,
nunca como 503 que prometa no consumo. No se simula una partición de red durante
COMMIT: se prueba el mapeo conservador y los rollback antes de commit.
503 por protección, conexión previa a commit o firma no borra el refresh y no
equivale a revocación. Resultado ambiguo obliga a nuevo login.

Reutiliza Redis, Lua y resolución de IP confiable. Valores operativos iniciales
**propuestos y configurables**, no política aprobada de bloqueo administrativo:

| Presupuesto | Capacidad / recarga completa |
|---|---|
| Login por IP | 6 / 60 s |
| Refresh por IP | 30 / 60 s |
| Logout por IP | 30 / 60 s |
| Login por correo normalizado, identificado con HMAC | 10 / 900 s |

El presupuesto por correo consume todos los intentos, exitosos o fallidos, sin
consultar existencia de cuenta; limita fallos sin implementar bloqueo permanente
o contadores públicos por cuenta. Usa dominio HMAC distinto del reenvío de correo.
Umbrales elegidos para acotar Argon2 y abuso, sujetos a revisión y mediciones.
Redis indisponible rechaza las operaciones nuevas con 503 antes de consumir refresh.
La saturación del semáforo compartido devuelve 429, conservando registro.

Nginx añade rutas exactas con presupuestos propios (login 60/min, burst 12;
refresh/logout 120/min, burst 20) y Problem Details estable. Mantiene los prefijos
existentes de registro/verificación y no toca firewall, puertos o proxies confiables.
HTTPS en producción sigue siendo obligatorio; el gateway actual es HTTP de desarrollo.

## Validación reproducible

Usar PostgreSQL **aislado de pruebas** con CREATEDB y Redis de pruebas para el
conjunto completo. No usar credenciales o bases de producción. SQLx crea bases
por prueba, aplica migraciones y elimina únicamente esas bases.

```sh
cd services/auth
cargo fmt --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked -- --include-ignored
```

Sin Redis, ejecutar con DATABASE_URL del servidor PostgreSQL de pruebas:

```sh
cargo test --locked -- \
  --skip email_link_confirms_and_head_does_not_consume \
  --skip exhausted_recipient_budget_preserves_existing_verification_link
```

Los cuatro tests Redis marcados ignore se reservan para CI con `--include-ignored`.
Las pruebas HTTP de sesión usan bypass de presupuesto exclusivamente `cfg(test)`;
no existe ese bypass en builds de producción. El nuevo test Redis real verifica
presupuesto por cuenta y por IP en CI; no se presenta como ejecutado localmente.

Desde la raíz, Nginx real y backend desechable:

```sh
python3 scripts/test-auth-jwks-gateway.py --nginx /usr/sbin/nginx
```

La prueba comprueba rutas, límites y encabezados, no autenticación del backend stub.
No se inicia Core/Files ni se acredita interoperabilidad Rust–Elixir.

## Estado inicial y resultado final

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

Pendientes: #17 y su contrato de recuperación/enumeración; límites operativos
definitivos; pruebas Redis completas y Linux; limpieza/retención; publicación
RabbitMQ, API interna, consumidores e interoperabilidad. Siguiente entrega:
resolver #17 con contrato y pruebas antes de habilitar login, sin ampliar esta
entrega a recuperación completa ni integración con los compañeros.
