# `channel.deleted` v1 (Core)

Aviso de que un canal fue eliminado (CU-13 FA-02). Se entrega por el tema
personal [`user:<user_id>`](user.topic.md) de cada miembro del grupo cuyo rol
alcanzaba el `min_role` del canal.

## Evento

```json
{
  "type": "channel.deleted",
  "version": 1,
  "payload": {
    "id": "0b2a7c1e-52a6-4a41-8a43-0d2f3f5d9a11",
    "group_id": "4f1d1d4e-7a7a-4c7f-9d54-5a5a0b1f6a10"
  }
}
```

| Campo | Significado |
|---|---|
| `id` | Canal eliminado |
| `group_id` | Grupo del canal |

Además del aviso, Core cierra los temas `channel:<channel_id>` y
`voice:<channel_id>` para todos los conectados: el cliente recibe el cierre del
tema y no debe reconectarse. Las personas que estaban en la sala de voz son
sacadas de LiveKit de mejor esfuerzo, sin bloquear la eliminación
([`voice:<channel_id>`](voice.presence.md#expulsión)). El historial deja de
servirse y el nombre puede reutilizarse.
