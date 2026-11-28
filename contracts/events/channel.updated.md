# `channel.updated` v1 (Core)

Aviso de que cambió el nombre o la descripción de un canal (CU-13 FA-01).
Se entrega por el tema personal [`user:<user_id>`](user.topic.md) de cada
miembro del grupo cuyo rol alcanza el `min_role` del canal (D-17); quien no
puede ver el canal no lo recibe.

## Evento

```json
{
  "type": "channel.updated",
  "version": 1,
  "payload": {
    "id": "0b2a7c1e-52a6-4a41-8a43-0d2f3f5d9a11",
    "group_id": "4f1d1d4e-7a7a-4c7f-9d54-5a5a0b1f6a10",
    "name": "anuncios",
    "description": "Avisos del equipo",
    "type": "text",
    "min_role": "member"
  }
}
```

| Campo | Significado |
|---|---|
| `id` | Canal modificado |
| `group_id` | Grupo del canal |
| `name` | Nombre vigente |
| `description` | Descripción vigente o `null` |
| `type` | `text` o `voice` (no cambia) |
| `min_role` | Rol mínimo para ver el canal (no cambia) |

Core lo emite una vez confirmado el cambio (`PATCH
/api/v1/groups/{group_id}/channels/{channel_id}`) y solo si algún campo
cambió. El payload es el canal completo, no solo lo modificado.
