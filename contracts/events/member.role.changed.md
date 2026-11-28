# `member.role.changed` v1 (Core)

Aviso al miembro cuyo rol cambió (CU-14 paso 7). Se entrega por el tema
personal [`user:<user_id>`](user.topic.md); la notificación persistida
(CU-31) es un contrato aparte.

## Evento

```json
{
  "type": "member.role.changed",
  "version": 1,
  "payload": {
    "group_id": "4f1d1d4e-7a7a-4c7f-9d54-5a5a0b1f6a10",
    "role": "moderator",
    "previous_role": "member"
  }
}
```

| Campo | Significado |
|---|---|
| `group_id` | Grupo donde cambió el rol |
| `role` | Rol nuevo: `member`, `moderator` o `admin` |
| `previous_role` | Rol anterior |

Core lo emite una vez confirmado el cambio (`PATCH
/api/v1/groups/{group_id}/members/{user_id}`). Si el rol enviado es el que el
miembro ya tenía, no hay cambio y no se emite. El cliente debe volver a pedir
`GET /api/v1/groups/{group_id}` o la lista de miembros para ver su rol.
