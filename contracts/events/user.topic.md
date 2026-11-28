# Tema `user:<user_id>` v1 (Core)

Contrato candidato del canal personal de cada usuario en el WebSocket de Core
(RealtimeService). Por este tema llegan los avisos dirigidos a una sola
persona: cambio de rol (CU-14), moderación (CU-15) y notificaciones (CU-31).
Los eventos concretos se documentan en cada caso de uso; aquí se define el
tema y el sobre.

## Conexión

Igual que [`message.created`](message.created.md): WebSocket de Phoenix en
`/socket/websocket` con el access JWT de Auth como `authToken`.

## Tema `user:<user_id>`

Un usuario solo puede unirse a su propio tema, con el `user_id` del token
(`auth.accounts.id`). Respuesta de error de `join`:

| `reason` | Significado |
|---|---|
| `not_found` | El tema es de otro usuario o el id no es válido; no se distingue |
| `token_expired` | El access token ya venció; reconectar con uno nuevo |

Cuando vence el `exp` del token, Core envía `token.expired` (sin datos) y cierra
el tema. El cliente debe renovar el token y reconectar el socket.

## Sobre de los eventos

Todo evento del tema usa el sobre de [`README.md`](README.md):

```json
{
  "type": "member.role.changed",
  "version": 1,
  "payload": {}
}
```

El nombre del evento de Phoenix es igual a `type`. Un cliente ignora los
eventos o versiones que no conoce.

## Pérdida de acceso a un grupo

Cuando un usuario deja de tener acceso a un grupo (expulsión o bloqueo,
CU-15), Core cierra sus canales `channel:<channel_id>` y `voice:<channel_id>`
de los canales de ese grupo. El socket y los temas de otros grupos siguen
abiertos. El cliente recibe el cierre del canal y no debe reconectarse a esos
temas hasta recuperar el acceso.
