# `group.deleted` v1 (Core)

Aviso de que el grupo fue eliminado (CU-09 FA-02). Se entrega por el tema personal [`user:<user_id>`](user.topic.md) de
cada miembro del grupo.

## Evento

```json
{
  "type": "group.deleted",
  "version": 1,
  "payload": {
    "id": "4f1d1d4e-7a7a-4c7f-9d54-5a5a0b1f6a10"
  }
}
```

| Campo | Significado |
|---|---|
| `id` | Grupo eliminado |

Core lo emite una vez confirmada la eliminación (`DELETE /api/v1/groups/{group_id}`). Además cierra los temas `channel:<channel_id>` y `voice:<channel_id>` de todos los canales del grupo para todos los conectados y saca de LiveKit, de mejor esfuerzo, a quienes estaban en las salas de voz. El grupo deja de servirse para todos los miembros (`404`); el cliente debe quitarlo de su lista y no reconectar sus temas.
