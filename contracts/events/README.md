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

## Eventos entre servicios (RabbitMQ, D-04)

- *Exchange* `topic`: `sonimbus.events`. La clave de ruteo es el nombre del evento.
- Una cola durable por consumidor y una cola de mensajes fallidos.
- Cada evento se documenta aquí con: nombre, versión, productor, consumidores y esquema JSON del `payload`.

| Evento | Productor | Consumidores | Estado |
|---|---|---|---|
| `user.updated` | Auth | Core, Files, Reports | Por definir |
| `message.created` | Core | Reports | Por definir |
| `task.updated` | Core | Reports | Por definir |
| `member.joined` | Core | Reports | Por definir |

Ver las reglas en [`contracts/README.md`](../README.md).
