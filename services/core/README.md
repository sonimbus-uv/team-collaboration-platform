# Core Service

**Responsable principal:** `@Maple-M136279841`

## Responsabilidad

- Grupos, miembros, roles y permisos.
- Proyectos y tareas.
- Mensajería, reacciones e historial.
- Reuniones de voz (estado y metadatos; el audio lo maneja LiveKit).
- Notificaciones.
- Canales WebSocket hacia los clientes.

Los dominios deben mantenerse desacoplados entre sí para facilitar su posible separación en servicios independientes.

## Stack

- Elixir, Phoenix y Erlang/OTP.
- PostgreSQL, en su propia base de datos, para información estructurada.
- ScyllaDB para mensajes, historial y actividad.
- Redis para presencia y caché.
- RabbitMQ para eventos entre servicios.

## Fuera de alcance

- Identidad, credenciales y perfil de usuario: pertenecen a Auth.
- Contenido de los archivos: pertenece al File Service; Core solo guarda la referencia.

## Contratos

- Expone: [`contracts/openapi/core.yaml`](../../contracts/openapi/) y los eventos WebSocket de [`contracts/events/`](../../contracts/events/).

## Pendiente de decidir

- Qué servicio emite los tokens de acceso a las salas de LiveKit.

## Base de datos

Las migraciones de PostgreSQL viven dentro de este servicio (Ecto). Los esquemas CQL de ScyllaDB viven en [`database/scylla/`](../../database/scylla/).

## Estado

Sin inicializar. El primer PR del responsable crea el proyecto (`mix phx.new`) con las versiones exactas de su entorno y actualiza este README con los pasos para compilar y ejecutar.
