# `member.left` v1 (Core)

Aviso de que un miembro abandonó el grupo (CU-12). Se entrega por el tema
personal [`user:<user_id>`](user.topic.md) de cada miembro que sigue en el
grupo; quien se fue no lo recibe.

## Evento

```json
{
  "type": "member.left",
  "version": 1,
  "payload": {
    "group_id": "4f1d1d4e-7a7a-4c7f-9d54-5a5a0b1f6a10",
    "user_id": "9a0d2b1c-3f55-4d0e-8f0e-2b6f1c1d7a42"
  }
}
```

| Campo | Significado |
|---|---|
| `group_id` | Grupo que se abandonó |
| `user_id` | Usuario que se fue (`auth.accounts.id`) |

Core lo emite una vez confirmada la salida (`DELETE
/api/v1/groups/{group_id}/members/me`). Los mensajes del usuario permanecen en
los canales. Además, Core cierra los temas `channel:<channel_id>` y
`voice:<channel_id>` del grupo para ese usuario
([`user:<user_id>`](user.topic.md#pérdida-de-acceso-a-un-grupo)) y pide a
LiveKit sacarlo de las salas de voz, de mejor esfuerzo. Si el usuario vuelve
con una invitación, entra como `member`. El cliente de quien se fue no recibe
este evento: debe quitar el grupo de su lista al recibir `204`.
