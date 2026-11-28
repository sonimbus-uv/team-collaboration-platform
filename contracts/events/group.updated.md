# `group.updated` v1 (Core)

Aviso de que cambió el nombre, la descripción o la imagen del grupo (CU-09). Se entrega por el tema personal [`user:<user_id>`](user.topic.md) de
cada miembro del grupo.

## Evento

```json
{
  "type": "group.updated",
  "version": 1,
  "payload": {
    "id": "4f1d1d4e-7a7a-4c7f-9d54-5a5a0b1f6a10",
    "name": "Equipo expo",
    "description": "Coordinación de la expo",
    "image_id": null,
    "archived_at": null
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

Core lo emite una vez confirmado el cambio (`PATCH /api/v1/groups/{group_id}`) y solo si algún campo cambió. El payload es el grupo completo; el rol de cada miembro se obtiene con `GET /api/v1/groups/{group_id}`.
