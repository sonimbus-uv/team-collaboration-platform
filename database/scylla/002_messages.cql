-- Mensajes por canal (CU-16, CU-17, CU-19, CU-20, CU-23).
--
-- Particion (channel_id, bucket): un bucket por mes UTC, como entero AAAAMM
-- (por ejemplo, 202610). El bucket se calcula a partir de message_id.
-- message_id: timeuuid generado por Core al guardar; define el orden (D-08).
-- Historial: la pagina mas reciente sale del bucket actual con LIMIT; el
-- cursor es el ultimo message_id entregado (message_id < ?). Si la pagina no
-- se llena, Core sigue con el mes anterior, hasta el mes de channels.created_at.
--
-- Eliminar no usa DELETE (evita tombstones): deleted = true, con
-- deleted_reason 'author' o 'moderation' y deleted_by (D-24). Core no entrega
-- content ni file_id de un mensaje eliminado.

CREATE TABLE IF NOT EXISTS sonimbus_core.messages_by_channel (
    channel_id        uuid,
    bucket            int,
    message_id        timeuuid,
    author_id         uuid,       -- auth.accounts.id, sin FK
    client_message_id uuid,       -- idempotencia (003_messages_by_client_id.cql)
    content           text,       -- 1-4000 caracteres, validado en Core (D-22)
    reply_to          timeuuid,   -- CU-17; mismo canal, su bucket sale del timeuuid
    file_id           uuid,       -- CU-23; archivo del File Service, sin FK
    edited_at         timestamp,  -- NULL = nunca editado
    deleted           boolean,
    deleted_reason    text,       -- 'author' | 'moderation'
    deleted_by        uuid,
    PRIMARY KEY ((channel_id, bucket), message_id)
) WITH CLUSTERING ORDER BY (message_id DESC);
