# Contratos de eventos

Eventos WebSocket hacia los clientes y eventos entre servicios por RabbitMQ.

Formato de nombre: `<recurso>.<evento>` (por ejemplo, `message.created`). Cada evento declara su versión:

```json
{
  "type": "message.created",
  "version": 1,
  "payload": {}
}
```

Ver las reglas en [`contracts/README.md`](../README.md).
