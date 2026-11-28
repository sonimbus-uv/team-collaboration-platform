# `message.updated` v1 (Core)

Se difunde a todos los suscriptores de `channel:<channel_id>`, incluido el
autor, cuando el autor edita un mensaje (CU-19). Mismo sobre y conexión que
[`message.created`](message.created.md).

```json
{
  "type": "message.updated",
  "version": 1,
  "payload": {
    "id": "0955f155-d37a-11f1-bf48-3f32a97dd8eb",
    "channel_id": "c3349769-ec75-4913-9af0-d405da793293",
    "author_id": "11111111-1111-4111-8111-111111111111",
    "author": { "id": "11111111-1111-4111-8111-111111111111", "display_name": "Ana" },
    "client_message_id": "8f0e7c52-3a59-4f7e-9d43-4a5b7a3c2e10",
    "content": "Hola a todos",
    "reply_to": null,
    "reply_preview": null,
    "file_id": null,
    "created_at": "2026-10-29T15:04:05.123456Z",
    "edited_at": "2026-10-29T15:10:00.000000Z",
    "deleted": false,
    "deleted_reason": null
  }
}
```

- El `payload` es el mensaje completo, con la misma forma que `message.created`
  y el historial; `edited_at` ya no es `null`.
- Los clientes reemplazan el mensaje con ese `id`. Si no lo tienen cargado,
  lo ignoran.
