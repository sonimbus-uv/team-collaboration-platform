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

## Eventos WebSocket hacia los clientes

| Evento | Productor | Estado |
|---|---|---|
| [`message.created` v1](message.created.md) | Core | Candidato; implementado en Core para la expo |
| [`message.updated` v1](message.updated.md) | Core | Candidato; edición por el autor |
| [`message.deleted` v1](message.deleted.md) | Core | Candidato; eliminación por el autor o por moderación |
| [`user:<user_id>` v1](user.topic.md) | Core | Candidato; tema personal, eventos concretos por caso de uso |
| [`voice:<channel_id>` v1](voice.presence.md) | Core | Candidato; participantes, micrófono y cámara con Phoenix Presence |
| [`member.role.changed` v1](member.role.changed.md) | Core | Candidato; por `user:<user_id>` |
| [`moderation.applied` v1](moderation.applied.md) | Core | Candidato; por `user:<user_id>` |

## Eventos entre servicios (RabbitMQ, D-04)

- *Exchange* `topic`: `sonimbus.events`. La clave de ruteo es el nombre del evento.
- Una cola durable por consumidor y una cola de mensajes fallidos.
- Cada evento se documenta aquí con: nombre, versión, productor, consumidores y esquema JSON del `payload`.

| Evento | Productor | Consumidores | Estado |
|---|---|---|---|
| [`session.revoked` v1](session.revoked.md) | Auth | Core, Files | Diseño #35; implementación e integración pendientes |
| `user.updated` | Auth | Core, Files, Reports | Por definir |
| `message.created` | Core | Reports | Por definir |
| `task.updated` | Core | Reports | Por definir |
| `member.joined` | Core | Reports | Por definir |

Ver las reglas en [`contracts/README.md`](../README.md).
