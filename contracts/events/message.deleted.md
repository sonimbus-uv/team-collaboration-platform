# `message.deleted` v1 (Core)

Se difunde a todos los suscriptores de `channel:<channel_id>` cuando un mensaje
se elimina: por su autor (CU-19 FA-01) o por moderación (CU-15 FA-01). Mismo
sobre y conexión que [`message.created`](message.created.md).

```json
{
  "type": "message.deleted",
  "version": 1,
  "payload": {
    "id": "0955f155-d37a-11f1-bf48-3f32a97dd8eb",
    "channel_id": "c3349769-ec75-4913-9af0-d405da793293",
    "deleted_reason": "author"
  }
}
```

| Campo | Significado |
|---|---|
| `id` | Mensaje eliminado |
| `channel_id` | Canal del mensaje |
| `deleted_reason` | `author` ("Mensaje eliminado") o `moderation` ("Mensaje eliminado por moderación") |

El mensaje no desaparece del historial: se entrega con `deleted = true`,
`content` y `file_id` nulos y `deleted_reason`. Los clientes lo sustituyen por
el aviso correspondiente y retiran sus adjuntos.
