# Insumos para el ERD del esquema `core`

Datos reunidos el 2026-10-27 para dibujar el diagrama entidad-relación de Core.
Las decisiones que los justifican están en
[core-decisions-draft.md](core-decisions-draft.md) (D-17 a D-28).

Convenciones:

- Esquema `core` en PostgreSQL, usuario `core_service` (D-13).
- Claves primarias `uuid`, salvo las tablas de unión con clave compuesta.
- Horas en `timestamptz` (UTC).
- `user_id`, `actor_id`, `recipient_id`, `assignee_id`, `created_by` y columnas
  similares son `auth.accounts.id`: **sin FK**. En el ERD se dibujan como
  referencia externa a `user_profiles` o a Auth, con línea punteada.
- Los mensajes y reacciones viven en ScyllaDB, no en este esquema. En el ERD
  van como entidad externa referida por `channel_id`, sin FK.

## Tipos enumerados

| Tipo | Valores | Uso |
|---|---|---|
| `group_role` | `member`, `moderator`, `admin` (en ese orden) | `group_members.role`, `group_role_permissions.role`, `channels.min_role` |
| `channel_type` | `text`, `voice` | `channels.type` |
| `task_status` | `pending`, `in_progress`, `done` | `tasks.status` |
| `account_status` | `active`, `suspended`, `deleted` | `user_profiles.account_status` |
| `moderation_action` | `delete_message`, `kick`, `ban`, `unban` | `moderation_actions.action` |

El catálogo de permisos (`permission`) es una lista cerrada en código, validada
con `CHECK`. Propuesta inicial: `group.manage`, `roles.manage`, `channels.manage`,
`invitations.manage`, `members.moderate`, `messages.moderate`, `projects.manage`,
`tasks.assign`.

## Entidades

### CommunityService

**groups**

| Columna | Tipo | Restricciones |
|---|---|---|
| `id` | uuid | PK |
| `name` | text | NOT NULL, 2–100 caracteres |
| `description` | text | NULL, ≤ 500 |
| `image_id` | uuid | NULL; archivo de Files, sin FK |
| `created_by` | uuid | NOT NULL; usuario, sin FK |
| `archived_at` | timestamptz | NULL = activo; con fecha = solo lectura |
| `deleted_at` | timestamptz | NULL = vigente |
| `created_at`, `updated_at` | timestamptz | NOT NULL |

**group_members**

| Columna | Tipo | Restricciones |
|---|---|---|
| `group_id` | uuid | PK (1/2), FK → `groups.id` |
| `user_id` | uuid | PK (2/2); usuario, sin FK |
| `role` | group_role | NOT NULL, por defecto `member` |
| `joined_at` | timestamptz | NOT NULL |

Índice adicional `(user_id)` para listar los grupos de un usuario.

**group_role_permissions**

| Columna | Tipo | Restricciones |
|---|---|---|
| `group_id` | uuid | PK (1/3), FK → `groups.id` |
| `role` | group_role | PK (2/3) |
| `permission` | text | PK (3/3), `CHECK` contra el catálogo |
| `enabled` | boolean | NOT NULL |

**channels**

| Columna | Tipo | Restricciones |
|---|---|---|
| `id` | uuid | PK |
| `group_id` | uuid | NOT NULL, FK → `groups.id` |
| `name` | text | NOT NULL, 1–80, `^[a-z0-9-]+$` |
| `description` | text | NULL, ≤ 500 |
| `type` | channel_type | NOT NULL |
| `min_role` | group_role | NOT NULL, por defecto `member` |
| `deleted_at` | timestamptz | NULL = vigente |
| `created_at`, `updated_at` | timestamptz | NOT NULL |

Índice único parcial `(group_id, name) WHERE deleted_at IS NULL`.

**invitations**

| Columna | Tipo | Restricciones |
|---|---|---|
| `id` | uuid | PK |
| `group_id` | uuid | NOT NULL, FK → `groups.id` |
| `code` | text | NOT NULL, UNIQUE |
| `created_by` | uuid | NOT NULL; usuario, sin FK |
| `expires_at` | timestamptz | NOT NULL; por defecto ahora + 7 días, de 1 h a 30 días |
| `max_uses` | integer | NULL = ilimitado; `CHECK (max_uses BETWEEN 1 AND 100)` |
| `uses` | integer | NOT NULL, por defecto 0, `CHECK (uses >= 0)` |
| `revoked_at` | timestamptz | NULL = vigente |
| `created_at` | timestamptz | NOT NULL |

`CHECK (max_uses IS NULL OR uses <= max_uses)`.

**group_bans**

| Columna | Tipo | Restricciones |
|---|---|---|
| `group_id` | uuid | PK (1/2), FK → `groups.id` |
| `user_id` | uuid | PK (2/2); usuario, sin FK |
| `banned_by` | uuid | NOT NULL; usuario, sin FK |
| `reason` | text | NULL |
| `created_at` | timestamptz | NOT NULL |

**moderation_actions**

| Columna | Tipo | Restricciones |
|---|---|---|
| `id` | uuid | PK |
| `group_id` | uuid | NOT NULL, FK → `groups.id` |
| `actor_id` | uuid | NOT NULL; usuario, sin FK |
| `target_user_id` | uuid | NOT NULL; usuario, sin FK |
| `action` | moderation_action | NOT NULL |
| `target_ref` | text | NULL; por ejemplo, `channel_id:message_id` del mensaje en ScyllaDB |
| `reason` | text | NULL |
| `created_at` | timestamptz | NOT NULL |

Índice `(group_id, created_at DESC)`. Solo se agregan filas.

### ProjectService

**projects**

| Columna | Tipo | Restricciones |
|---|---|---|
| `id` | uuid | PK |
| `group_id` | uuid | NOT NULL, FK → `groups.id` |
| `name` | text | NOT NULL, 2–100 |
| `description` | text | NULL, ≤ 500 |
| `target_date` | date | NULL |
| `created_by` | uuid | NOT NULL; usuario, sin FK |
| `archived_at` | timestamptz | NULL = activo |
| `created_at`, `updated_at` | timestamptz | NOT NULL |

Índice único parcial `(group_id, name) WHERE archived_at IS NULL`.

**tasks**

| Columna | Tipo | Restricciones |
|---|---|---|
| `id` | uuid | PK |
| `project_id` | uuid | NOT NULL, FK → `projects.id` |
| `title` | text | NOT NULL, 1–200 |
| `description` | text | NULL |
| `status` | task_status | NOT NULL, por defecto `pending` |
| `assignee_id` | uuid | NULL; usuario, sin FK |
| `due_date` | date | NULL |
| `created_by` | uuid | NOT NULL; usuario, sin FK |
| `version` | integer | NOT NULL, por defecto 1 (bloqueo optimista) |
| `status_changed_at` | timestamptz | NULL |
| `created_at`, `updated_at` | timestamptz | NOT NULL |

Índices `(project_id, status)` y `(assignee_id)`.

### NotificationService

**notifications**

| Columna | Tipo | Restricciones |
|---|---|---|
| `id` | uuid | PK |
| `recipient_id` | uuid | NOT NULL; usuario, sin FK |
| `group_id` | uuid | NULL, FK → `groups.id` |
| `type` | text | NOT NULL (`message.reply`, `task.assigned`, `role.changed`, `moderation.applied`, `voice.activity`) |
| `payload` | jsonb | NOT NULL; solo referencias, sin datos sensibles |
| `read_at` | timestamptz | NULL = no leída |
| `created_at` | timestamptz | NOT NULL |

Índices `(recipient_id, created_at DESC)` y parcial `(recipient_id) WHERE read_at IS NULL`.

### Soporte

**user_profiles** (copia local de Auth, D-20)

| Columna | Tipo | Restricciones |
|---|---|---|
| `user_id` | uuid | PK; `auth.accounts.id`, sin FK |
| `display_name` | text | NOT NULL |
| `avatar_id` | text | NULL |
| `account_status` | account_status | NOT NULL |
| `version` | bigint | NOT NULL; solo se aplica una versión mayor |
| `updated_at` | timestamptz | NOT NULL |

**outbox_events**

| Columna | Tipo | Restricciones |
|---|---|---|
| `event_id` | uuid | PK; estable entre reintentos |
| `type` | text | NOT NULL; clave de ruteo |
| `version` | integer | NOT NULL |
| `correlation_id` | uuid | NULL |
| `occurred_at` | timestamptz | NOT NULL |
| `payload` | jsonb | NOT NULL |
| `published_at` | timestamptz | NULL = pendiente |

Índice parcial `(occurred_at) WHERE published_at IS NULL`.

**processed_events**

| Columna | Tipo | Restricciones |
|---|---|---|
| `event_id` | uuid | PK |
| `type` | text | NOT NULL |
| `processed_at` | timestamptz | NOT NULL |

Índice `(processed_at)`; se borra lo anterior a 30 días.

## Relaciones y cardinalidad

| Desde | Hacia | Cardinalidad | Tipo |
|---|---|---|---|
| `groups` | `group_members` | 1 : 1..N (siempre al menos un `admin`) | FK |
| `groups` | `group_role_permissions` | 1 : N | FK |
| `groups` | `channels` | 1 : 0..N | FK |
| `groups` | `invitations` | 1 : 0..N | FK |
| `groups` | `group_bans` | 1 : 0..N | FK |
| `groups` | `moderation_actions` | 1 : 0..N | FK |
| `groups` | `projects` | 1 : 0..N | FK |
| `projects` | `tasks` | 1 : 0..N | FK |
| `groups` | `notifications` | 1 : 0..N (opcional) | FK |
| `user_profiles` | `group_members`, `group_bans`, `tasks.assignee_id`, `notifications.recipient_id`, … | 1 : N | lógica, sin FK |
| `channels` | mensajes en ScyllaDB `(channel_id, bucket)` | 1 : N | externa, sin FK |

Las FK internas se declaran sin `ON DELETE CASCADE`: grupos y canales se borran
de forma lógica (D-19).

## Fuera del ERD de PostgreSQL

- Mensajes y reacciones: ScyllaDB (`docs/database/README.md`); con D-24, los
  mensajes agregan `deleted`, `deleted_reason` (`author` | `moderation`) y
  `deleted_by`.
- Presencia y caché de permisos: Redis.
- Metadatos de archivos: esquema `files` (File Service).
