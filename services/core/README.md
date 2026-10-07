# Core Service

**Responsable principal:** `@Maple-M136279841`

## Responsabilidad

Dominio principal de Sonimbus, organizado en servicios internos desacoplados para facilitar su posible separación:

| Servicio interno | Responsabilidad | Casos de uso |
|---|---|---|
| CommunityService | Grupos, membresías, invitaciones, canales, roles, permisos y moderación; expone `checkAccess` a los demás servicios | CU-08 a CU-15 |
| MessagingService | Mensajes, respuestas, reacciones, edición, eliminación e historial | CU-16 a CU-20 |
| RealtimeService | Canales WebSocket (Phoenix) hacia los clientes y presencia | CU-16, CU-31 |
| ProjectService | Proyectos y tareas con control optimista por versión | CU-25 a CU-28 |
| NotificationService | Notificaciones internas | CU-31 |
| VoiceService | Plano de control de la voz: permisos, participantes y emisión del token de LiveKit (D-15); el audio lo maneja LiveKit | CU-22 |

Además publica en el mensaje la referencia a los archivos que guarda el File Service (CU-23). Ver la trazabilidad completa en [`docs/requirements/`](../../docs/requirements/README.md#core-service).

## Stack

- Elixir, Phoenix y Erlang/OTP.
- PostgreSQL, esquema `core` con su propio usuario (D-13).
- ScyllaDB para mensajes y reacciones (D-05), con el driver Xandra.
- Redis para presencia y caché de permisos.
- RabbitMQ para eventos entre servicios (D-04): `Broadway` + `broadway_rabbitmq` para consumir y `AMQP` para publicar.
- `Joken` para el token de LiveKit.

## Fuera de alcance

- Identidad, credenciales y perfil de usuario: pertenecen a Auth. Core guarda solo una copia de nombre y foto, actualizada por `user.updated` (D-06).
- Contenido y metadatos de los archivos: pertenecen al File Service; Core solo guarda la referencia.
- Agregados de reportes: pertenecen al Report Service.

## Contratos

- Expone: `contracts/openapi/core.yaml`, los eventos WebSocket de [`contracts/events/`](../../contracts/events/) y `checkAccess` por gRPC.
- Publica eventos de dominio en RabbitMQ (`message.created`, `task.updated`, `member.joined`, ...).

## Base de datos

- Las migraciones de PostgreSQL viven dentro de este servicio (Ecto).
- Los esquemas CQL de ScyllaDB viven en [`database/scylla/`](../../database/scylla/).
- El diseño de tablas está en [`docs/database/`](../../docs/database/README.md#scylladb-d-05).

## Estado

Sin inicializar. El primer PR del responsable crea el proyecto (`mix phx.new`) con las versiones exactas de su entorno y actualiza este README con los pasos para compilar y ejecutar.
