# Presencia en canales de voz v1 (Core)

Contrato candidato de los participantes de un canal de voz (CU-22, RF-VOZ-01,
RF-VOZ-02). Core lleva la lista de participantes y el estado de su micrófono y
cámara; el audio y el vídeo viajan por LiveKit, con el token de
[`createVoiceToken`](../openapi/core.yaml). Core no ve ni procesa los medios.

## Conexión

Igual que [`message.created`](message.created.md): WebSocket de Phoenix en
`/socket/websocket` con el access JWT de Auth como `authToken`.

## Tema `voice:<channel_id>`

Solo canales de voz. Unirse exige ser miembro del grupo con un rol igual o
superior a `min_role` del canal (D-17) y que el grupo no esté archivado.

Carga opcional del `join`; ambos campos son booleanos y valen `false` por
omisión (el miembro entra en modo solo escucha, CU-22 FA-02):

```json
{ "mic": true, "camera": false }
```

Respuesta de error de `join`:

| `reason` | Significado |
|---|---|
| `not_found` | El canal no existe, es de texto o el usuario no tiene acceso; no se distingue |
| `group_archived` | El grupo es de solo lectura |
| `invalid_payload` | `mic` o `camera` no son booleanos |
| `token_expired` | El access token ya venció; reconectar con uno nuevo |

Al vencer el `exp` del token, Core envía `token.expired` y cierra el tema.

## Participantes: eventos estándar de Phoenix Presence

La clave de cada participante es su `user_id`. Los eventos son los de
`Phoenix.Presence`, que los clientes de Phoenix ya manejan:

- `presence_state`: lista completa al unirse.
- `presence_diff`: `{"joins": {...}, "leaves": {...}}` cuando alguien entra,
  sale o cambia su estado.

Cada entrada de `metas`:

| Campo | Significado |
|---|---|
| `display_name` | Nombre visible vigente, o `null` si no tiene |
| `mic` | `true` si el micrófono está activo |
| `camera` | `true` si la cámara está activa |
| `joined_at` | Fecha de entrada, ISO 8601 en UTC |

Un mismo usuario con varias conexiones aparece con varias entradas en `metas`.

Al salir, cerrarse el socket o perder la conexión (CU-22 EX-02), el
participante deja de aparecer: llega un `presence_diff` con sus `leaves`.

## Cliente → servidor: `voice.update`

```json
{ "mic": false, "camera": true }
```

Cambia el estado del micrófono y la cámara (CU-22 FA-01). Los campos son
opcionales, pero debe venir al menos uno y deben ser booleanos.
Respuesta `ok`: `{}`. Respuesta `error`: `{"type": "validation-error",
"errors": {"mic": ["must be a boolean"]}}`.

## Expulsión

Si el usuario es expulsado o bloqueado (CU-15), Core cierra su tema
`voice:<channel_id>` y deja de ser participante (CU-22 FA-03); ver
[`user:<user_id>`](user.topic.md#pérdida-de-acceso-a-un-grupo).

Además, Core pide a LiveKit sacarlo de la sala de cada canal de voz del grupo
(`RemoveParticipant`, con room = id del canal e identity = `user_id`). Es de
mejor esfuerzo y no bloquea la moderación: si LiveKit no responde o no está
configurado, la expulsión o el bloqueo se aplican igual y no hay reintentos
duraderos. LiveKit no revoca tokens ya emitidos, así que un token vigente
(hasta `expires_at`) podría volver a entrar a la sala; Core no emite tokens
nuevos para quien ya no es miembro.
