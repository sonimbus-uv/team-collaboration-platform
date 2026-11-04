-- Reacciones (CU-18, escenario de concurrencia 3).
--
-- Una fila por (mensaje, emoji, usuario): la clave impide duplicados. Sin tipo
-- counter, porque no es idempotente; los totales se cuentan al leer.
-- Misma particion que messages_by_channel: las reacciones de una pagina del
-- historial salen en una sola consulta por rango de message_id.
--
-- Retirar una reaccion no usa DELETE: active = false. La API recibe el estado
-- deseado (agregar o retirar), no un "alternar", para que el reintento sea
-- idempotente.

CREATE TABLE IF NOT EXISTS sonimbus_core.reactions_by_channel (
    channel_id uuid,
    bucket     int,
    message_id timeuuid,
    emoji      text,       -- validado en Core
    user_id    uuid,       -- auth.accounts.id, sin FK
    active     boolean,
    updated_at timestamp,
    PRIMARY KEY ((channel_id, bucket), message_id, emoji, user_id)
) WITH CLUSTERING ORDER BY (message_id DESC, emoji ASC, user_id ASC);
