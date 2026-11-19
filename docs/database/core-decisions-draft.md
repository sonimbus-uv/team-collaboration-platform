# Decisiones de datos de Core (borrador)

Borrador del 2026-10-27, previo al ERD del esquema `core`. Las decisiones se
numeran como D-17 a D-30 y se agruparán en el ADR-009 (modelo de datos de Core).
D-16 y el ADR-007 son de las sesiones de Auth (#44) y el ADR-008 de los códigos
de registro (#47), ya fusionados en `main`.

Referencias: [casos de uso](../requirements/casos-de-uso.md),
[requisitos](../requirements/requisitos.md), [modelo de datos](README.md),
[decisiones](../decisions/README.md).

## Decisiones

| ID | Pregunta | Decisión | Consecuencia en el esquema |
|---|---|---|---|
| D-17 | ¿Canales privados o todo miembro ve todos los canales? | Canales con **rol mínimo**: cada canal define el rol mínimo para verlo y usarlo. Sin listas de miembros por canal en el MVP. | `channels.min_role` (por defecto `member`). `checkAccess` compara el rol de la membresía con `min_role`. Files y el historial consultan el mismo criterio (RF-MSG-01, RF-MSG-08). |
| D-18 | Permisos por rol (CU-14 FA-01) | Roles fijos y ordenados `member < moderator < admin`, más permisos ajustables por rol y grupo sobre un **catálogo cerrado** definido en código. | Tipo `group_role` ordenado (`member`, `moderator`, `admin`). Tabla `group_role_permissions (group_id, role, permission, enabled)`; los valores por defecto se siembran al crear el grupo. El catálogo no es tabla: un `CHECK` o validación en Ecto limita `permission`. |
| D-19 | Borrado de grupo y canal | **Borrado lógico** con `deleted_at`. Los mensajes en ScyllaDB quedan huérfanos y no se sirven porque `checkAccess` falla. | `groups.deleted_at`, `channels.deleted_at`. Unicidad de nombre de canal con índice parcial `WHERE deleted_at IS NULL`. Toda consulta filtra `deleted_at IS NULL`. Sin trabajo de purga en el MVP. |
| D-20 | Contenido de `user.updated` (lo produce Auth) | Core propone a Auth: `user_id`, `display_name`, `avatar_id`, `account_status` (`active`, `suspended`, `deleted`) y `version` por usuario. Motivo: en #44 una baja llega a Core solo como `session.revoked` con `reason = admin`. | Tabla `user_profiles (user_id PK, display_name, avatar_id, account_status, version, updated_at)`. Se aplica solo si `version` es mayor que la guardada. `user_id` es `auth.accounts.id`, sin FK (D-13). Pendiente: acordarlo con @tuzc0 en `contracts/events/`. |
| D-21 | Eventos en Core | *Outbox* para publicar e idempotencia para consumir. | `outbox_events (event_id PK, type, version, correlation_id, occurred_at, payload jsonb, published_at)` se escribe en la misma transacción que el cambio; un *worker* publica tras el commit. `processed_events (event_id PK, type, processed_at)` deduplica `user.updated` y `session.revoked`. |
| D-22 | Valores concretos | Mensaje: 1–4000 caracteres. Historial: página de 50, máximo 100. Invitación: caducidad de 7 días y usos ilimitados por defecto; el creador puede fijar de 1 h a 30 días y de 1 a 100 usos. Nombre de grupo: 2–100. Nombre de canal: 1–80, minúsculas, dígitos y guiones. Descripción: hasta 500. | `CHECK` de longitud en `groups.name`, `channels.name` y las descripciones. `invitations.expires_at` y `max_uses` (`NULL` = ilimitado) con `CHECK (max_uses BETWEEN 1 AND 100)`. La longitud del mensaje y el tamaño de página se validan en Core y viven en configuración. |
| D-23 | Notificaciones (CU-31) | Se conservan en PostgreSQL. Las leídas con más de 90 días se pueden limpiar con un trabajo opcional. | `notifications (id, recipient_id, group_id, type, payload jsonb, read_at, created_at)`. Índice `(recipient_id, created_at DESC)` e índice parcial `(recipient_id) WHERE read_at IS NULL` para el contador. |
| D-24 | Moderación (CU-15) | Marca en el mensaje dentro de ScyllaDB y auditoría en PostgreSQL. | ScyllaDB: `deleted`, `deleted_reason` (`author` o `moderation`) y `deleted_by` en la tabla de mensajes. PostgreSQL: `moderation_actions (id, group_id, actor_id, target_user_id, action, target_ref, reason, created_at)` y `group_bans (group_id, user_id, banned_by, reason, created_at)` para FA-03 y FA-04. |
| D-25 | Sobre de eventos | Igual que `session.revoked` de #44 más `correlation_id`: `type`, `version`, `event_id` (UUID estable entre reintentos), `occurred_at` (UTC), `correlation_id` y `payload`. Exchange *topic* `sonimbus.events`, clave de ruteo igual a `type`. | Columnas del mismo nombre en `outbox_events`. Pendiente: proponer `correlation_id` a Auth en el catálogo de eventos, opcional para que `session.revoked` v1 siga siendo válido. |
| D-26 | Retención de `processed_events` | 30 días: 20 de vida máxima de sesión más 10 de margen para reintentos y reproceso desde la DLQ. | Índice por `processed_at`; un trabajo diario borra lo anterior a `now() - interval '30 days'`. Las *tombstones* de sesiones revocadas siguen su propia regla (#44, hasta la expiración de la sesión). |
| D-27 | Numeración | Core usa D-17 en adelante y un solo ADR-009 (el ADR-008 lo tomó Auth en #47). | Ninguna. |
| D-28 | T-03 RabbitMQ y TLS | Un usuario AMQP por servicio con permisos mínimos; cola durable `core.session-revoked.v1` con DLQ en `definitions.json`. AMQP y HTTP interno **sin TLS hasta después de la expo** (29 de octubre), dentro de la red `backend` aislada. | Ninguna en PostgreSQL. Es una desviación temporal de #44, que pide AMQP con TLS; se registra como tarea con fecha posterior al 29 de octubre (mkcert, puerto 5671). |
| D-29 | Nombre visible para la expo (29 de octubre) | Auth no guarda nombre visible y el JWT no lleva datos de perfil (`contracts/token-profile.md`). Para la expo, el cliente envía `display_name` a Core y Core lo guarda en `user_profiles`. **Desviación temporal de D-20**: el nombre lo declara el usuario y no lo valida Auth. | `user_profiles` se llena con una operación de Core (`PUT /api/v1/me/profile`) en lugar del evento `user.updated`; `version` queda en 0 hasta que llegue el primer evento de Auth, que siempre gana. |
| D-30 | Comprobación de la sesión del JWT para la expo (29 de octubre) | Core verifica la firma Ed25519 con el JWKS de Auth y los claims de `contracts/token-profile.md` (`iss`, `aud` con `sonimbus-core`, `sub`, `sid`, `iat`, `exp`, `jti`), pero **no consulta la sesión en Auth** (`contracts/session-checks.md`). **Desviación temporal de D-16**: una sesión revocada o una cuenta bloqueada siguen entrando hasta que vence su access token, como máximo 15 minutos. | Ninguna en PostgreSQL. Falta el cliente HTTPS de la API interna de Auth con su credencial de servicio y la microcaché de 10 s; después, aplicar `session.revoked` (D-21). |

## Entidades e invariantes por servicio interno

Todas las tablas viven en el esquema `core`. Las horas se guardan en UTC
(`timestamptz`). `user_id` es siempre `auth.accounts.id`, sin FK.

### CommunityService (grupos, canales, membresías, roles, invitaciones, moderación)

| Entidad | Invariantes |
|---|---|
| `groups` | Nombre de 2–100 caracteres. `archived_at` deja el grupo en solo lectura (CU-09 FA-01). `deleted_at` lo oculta y revoca todo acceso (D-19). Quien lo crea queda como `admin` en la misma transacción. |
| `group_members (group_id, user_id)` | Clave primaria compuesta: sin membresías duplicadas (CU-11 EX-02). `role` es `member`, `moderator` o `admin`. Siempre hay al menos un `admin` en un grupo no borrado (CU-12 EX-01, CU-14 EX-01), verificado con bloqueo de fila al cambiar roles o salir. |
| `group_role_permissions` | Solo permisos del catálogo cerrado. Valores por defecto al crear el grupo: invitaciones para `moderator` y superiores; moderación para `moderator` y superiores; canales, proyectos y configuración para `admin`. `admin` conserva siempre la configuración del grupo y la gestión de roles, para que no pueda quedarse sin ellas. |
| `channels` | `type` es `text` o `voice`. Nombre único por grupo entre canales no borrados (índice parcial). `min_role` por defecto `member` (D-17). |
| `invitations` | `code` único. Válida si no tiene `revoked_at`, `expires_at > now()` y `uses < max_uses` (o `max_uses` es nulo). Al unirse, se agrega la membresía y se descuenta el uso en una sola transacción, con `UPDATE ... WHERE` condicionado (CU-11 paso 6). No se usa si el usuario tiene una fila en `group_bans`. |
| `group_bans (group_id, user_id)` | Impide el reingreso (CU-15 FA-03). Retirar el bloqueo borra la fila y deja la acción en `moderation_actions` (FA-04). |
| `moderation_actions` | Solo se agregan filas. El actor tiene un rol estrictamente superior al del afectado (CU-15 EX-01). |

### ProjectService (proyectos y tareas)

| Entidad | Invariantes |
|---|---|
| `projects` | Pertenece a un grupo. Nombre único por grupo entre proyectos no archivados. `archived_at` deja el proyecto y sus tareas en solo lectura (CU-25 FA-02). |
| `tasks` | `status` es `pending`, `in_progress` o `done`. `version` empieza en 1 y aumenta en cada cambio. La actualización exige que la versión enviada coincida con la guardada (`UPDATE ... WHERE version = $n`, RNF-CON-02); si no coincide, devuelve un conflicto. `assignee_id` debe ser miembro del grupo al asignarse (CU-27 EX-01). Solo el creador o un `admin` la elimina (CU-26 EX-03). Solo el responsable o un `admin` cambia el estado (CU-28 EX-01). |

### NotificationService

| Entidad | Invariantes |
|---|---|
| `notifications` | Una fila por destinatario. `read_at` solo pasa de nulo a una fecha, nunca se desmarca. Marcar todas como leídas es un solo `UPDATE` por destinatario (CU-31 FA-02). El elemento relacionado puede no existir ya (CU-31 EX-01): el `payload` guarda referencias, no copias, y no lleva datos sensibles. |

### Soporte (eventos y copias locales)

| Entidad | Invariantes |
|---|---|
| `user_profiles` | Copia local de los datos que publica Auth (D-06, D-20). Solo se aplica una `version` mayor que la guardada. Con `account_status = deleted`, el usuario se muestra anonimizado y no puede volver a unirse. |
| `outbox_events` | Se escribe en la misma transacción que el cambio de dominio. `event_id` es estable entre reintentos. Se marca `published_at` solo tras la confirmación de RabbitMQ (*publisher confirms*). |
| `processed_events` | `event_id` único: un evento repetido no se vuelve a aplicar. Se conserva 30 días (D-26). |

## Pendiente

- Acordar con @tuzc0 el contenido de `user.updated` (D-20) y `correlation_id` en el sobre (D-25).
- Registrar la tarea de TLS en AMQP y HTTP interno con fecha posterior a la expo (D-28).
- ERD del esquema `core` (otra sesión).
- Pasar las decisiones aprobadas a `docs/decisions/README.md` y al ADR-009.
- Sustituir D-29 por `user.updated` (D-20) cuando Auth guarde el nombre visible.
- Retirar D-30: consultar la sesión en Auth (`contracts/session-checks.md`) y consumir `session.revoked`.
