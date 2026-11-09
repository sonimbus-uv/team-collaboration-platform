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
e invalida los demás en una transacción. Las cuentas verificadas o bloqueadas
no se sobrescriben. La implementación no sanea automáticamente cuentas heredadas.

El orden de bloqueo es correo, cuenta e intento. La elegibilidad se revalida bajo
bloqueo. Los fallos de código se confirman antes de devolver el error HTTP; un
fallo SQL revierte la activación completa. Las sesiones heredadas afectadas se
revocan con la intención de outbox correspondiente.

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
