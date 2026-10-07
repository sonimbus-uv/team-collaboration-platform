# Contratos

Acuerdos entre los servicios y los clientes. Es el lugar donde se coordinan los integrantes: un cliente se programa contra el contrato, no contra el código del servicio.

| Carpeta | Contenido |
|---|---|
| [`openapi/`](openapi/) | Un archivo OpenAPI por servicio: `auth.yaml`, `core.yaml`, `files.yaml`, `reports.yaml`. Rutas bajo `/api/v1`. |
| [`events/`](events/) | Eventos WebSocket hacia los clientes y eventos entre servicios por RabbitMQ. |
| `proto/` | Archivos `.proto` de gRPC: `checkAccess` de Core y, si se elige, la transferencia de archivos. Se crea con el primer contrato. |

## Reglas

- Un cambio de contrato va en su propio PR, antes de implementarlo en el servicio o en los clientes.
- Todo cambio incompatible incrementa la versión del contrato o del evento.
- El PR debe indicar qué clientes y servicios se ven afectados.
- El dueño de cada servicio escribe y mantiene su contrato (ver [`docs/PLAN.md`](../docs/PLAN.md#reparto-de-módulos)).
- Errores con formato Problem Details y paginación por cursor, comunes a todos los servicios (secciones 13 y 14 de [`STANDARD_DEVELOPMENT.md`](../docs/STANDARD_DEVELOPMENT.md)).
