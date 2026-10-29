# Mensajes por WebSocket v1 (Core)

Contrato candidato del tiempo real de mensajes de Core (CU-16, CU-17),
pendiente de revisión técnica. El historial va por REST
(`GET /api/v1/channels/{channel_id}/messages`, [`core.yaml`](../openapi/core.yaml)).

## Conexión

- WebSocket de Phoenix en `/socket/websocket` (serializador JSON v2 del
  cliente oficial de Phoenix).
- El access JWT de Auth ([perfil](../token-profile.md), con `sonimbus-core` en
  `aud`) va como `authToken` del cliente de Phoenix, que lo envía en
  `Sec-WebSocket-Protocol` y no en la URL. Se acepta también el parámetro
  `token`, que Core filtra de sus registros.
- Token ausente o inválido: la conexión se rechaza.

## Tema `channel:<channel_id>`

Solo canales de texto. Unirse exige ser miembro del grupo con un rol igual o
superior a `min_role` (D-17). Respuesta de error de `join`:

| `reason` | Significado |
|---|---|
| `not_found` | El canal no existe, es de voz o el usuario no tiene acceso; no se distingue |
| `token_expired` | El access token ya venció; reconectar con uno nuevo |

Cuando vence el `exp` del token, Core envía `token.expired` (sin datos) y
cierra el canal. El cliente debe renovar el token y reconectar el socket.

## Cliente → servidor: `message.create`

```json
{
  "client_message_id": "8f0e7c52-3a59-4f7e-9d43-4a5b7a3c2e10",
  "content": "Hola, equipo",
  "reply_to": null
}
```

| Campo | Regla |
|---|---|
| `client_message_id` | UUID que genera el cliente por mensaje; se reutiliza en cada reintento (RNF-CON-01) |
| `content` | 1–4000 caracteres (D-22) |
| `reply_to` | Opcional: `id` de un mensaje del mismo canal (CU-17) |

Respuesta `ok`: `{"message": Message}`. Un reintento con el mismo
`client_message_id` devuelve el mismo mensaje, con el mismo `id`, y no se
difunde otra vez.

Respuesta `error`: `{"type": "<tipo>"}` y, en `validation-error`, `errors`
por campo.

| `type` | Significado |
|---|---|
| `validation-error` | Campos inválidos |
| `not-found` | El usuario perdió el acceso al canal |
| `group-archived` | El grupo está archivado (solo lectura) |
| `client-message-id-conflict` | Ese `client_message_id` ya se usó en otro canal |
| `token-expired` | El access token venció |
| `messages-unavailable` | ScyllaDB no responde; reintentar con el mismo `client_message_id` |

## Servidor → clientes: `message.created` v1

Se difunde a todos los suscriptores del tema, incluido el autor, después de
guardar el mensaje.

```json
{
  "type": "message.created",
  "version": 1,
  "payload": {
    "id": "0955f155-d37a-11f1-bf48-3f32a97dd8eb",
    "channel_id": "c3349769-ec75-4913-9af0-d405da793293",
    "author_id": "11111111-1111-4111-8111-111111111111",
    "client_message_id": "8f0e7c52-3a59-4f7e-9d43-4a5b7a3c2e10",
    "content": "Hola, equipo",
    "reply_to": null,
    "file_id": null,
    "created_at": "2026-10-29T15:04:05.123456Z",
    "edited_at": null,
    "deleted": false
  }
}
```

- `id` es un timeuuid que asigna Core y define el orden (D-08); `created_at`
  sale de él.
- Con `deleted = true`, `content` y `file_id` son `null` (D-24).
- Los clientes deduplican por `id` (el autor recibe la respuesta y la
  difusión).
