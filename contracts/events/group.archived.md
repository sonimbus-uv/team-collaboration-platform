# `group.archived` v1 (Core)

Aviso de que el grupo quedó archivado y en solo lectura (CU-09 FA-01). Se entrega por el tema personal [`user:<user_id>`](user.topic.md) de
cada miembro del grupo.

## Evento

```json
{
  "type": "group.archived",
  "version": 1,
  "payload": {
    "id": "4f1d1d4e-7a7a-4c7f-9d54-5a5a0b1f6a10",
    "name": "Equipo expo",
    "description": "Coordinación de la expo",
    "image_id": null,
    "archived_at": "2026-12-01T15:30:00Z"
  }
}
```

| Campo | Significado |
|---|---|
| `id` | Grupo |
| `name` | Nombre vigente |
| `description` | Descripción vigente o `null` |
| `image_id` | Archivo de imagen o `null` |
| `archived_at` | Instante de archivado o `null` |

Core lo emite una vez confirmado el archivado (`POST /api/v1/groups/{group_id}/archive`). Desde ese momento Core rechaza mensajes, canales nuevos, invitaciones y cambios de roles con `/problems/group-archived`; el historial sigue disponible y nadie pierde su membresía.
