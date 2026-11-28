# `reaction.updated` v1 (Core)

Se difunde a todos los suscriptores de `channel:<channel_id>`, incluido quien
reaccionó, cuando alguien agrega o retira una reacción (CU-18 paso 5). Mismo
sobre y conexión que [`message.created`](message.created.md).

```json
{
  "type": "reaction.updated",
  "version": 1,
  "payload": {
    "channel_id": "c3349769-ec75-4913-9af0-d405da793293",
    "message_id": "0955f155-d37a-11f1-bf48-3f32a97dd8eb",
    "emoji": "👍",
    "count": 3,
    "user_id": "11111111-1111-4111-8111-111111111111",
    "active": true
  }
}
```

| Campo | Significado |
|---|---|
| `message_id` | Mensaje reaccionado |
| `emoji` | Emoji de la reacción |
| `count` | Total de reacciones activas con ese emoji tras el cambio |
| `user_id` | Quien agregó o retiró la reacción |
| `active` | `true` si la agregó, `false` si la retiró |

Los clientes reemplazan el total de ese emoji por `count` (no suman ni restan)
y actualizan su propio estado cuando `user_id` es el usuario local. Si el
mensaje no está cargado, lo ignoran.
