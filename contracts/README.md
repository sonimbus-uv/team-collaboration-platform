# Contratos

Acuerdos entre los servicios y los clientes. Es el lugar donde se coordinan los integrantes: un cliente se programa contra el contrato, no contra el código del servicio.

| Carpeta | Contenido |
|---|---|
| [`openapi/`](openapi/) | Un archivo OpenAPI por servicio: `auth.yaml`, `core.yaml`, `files.yaml`. |
| [`events/`](events/) | Eventos WebSocket hacia los clientes y eventos entre servicios por RabbitMQ. |

## Reglas

- Un cambio de contrato va en su propio PR, antes de implementarlo en el servicio o en los clientes.
- Todo cambio incompatible incrementa la versión del contrato o del evento.
- El PR debe indicar qué clientes y servicios se ven afectados.
- Si se decide usar gRPC, los archivos `.proto` se agregan en `proto/`.
