# `moderation.applied` v1 (Core)

Aviso al afectado por una acción de moderación (CU-15 paso 7). Se entrega por
su tema personal [`user:<user_id>`](user.topic.md) justo después de aplicarla.
Si el usuario no está conectado no se reenvía; la notificación persistida
(CU-31) es un contrato aparte.

## Evento

```json
{
  "type": "moderation.applied",
  "version": 1,
  "payload": {
    "group_id": "4f1d1d4e-7a7a-4c7f-9d54-5a5a0b1f6a10",
    "action": "expel",
    "reason": "Spam"
  }
}
```

| Campo | Significado |
|---|---|
| `group_id` | Grupo donde se aplicó la acción |
| `action` | `expel` (expulsión) o `ban` (bloqueo) |
| `reason` | Motivo capturado por el moderador, o `null` |

El quién y el cuándo no se envían al afectado; quedan en el registro de
moderación del grupo. Retirar un bloqueo (`unban`) no genera este evento.

Además del aviso, Core cierra los temas `channel:<channel_id>` y
`voice:<channel_id>` del grupo para ese usuario
([`user:<user_id>`](user.topic.md#pérdida-de-acceso-a-un-grupo)).
