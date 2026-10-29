-- Idempotencia de envio (CU-16 FA-02, RNF-CON-01, escenario de concurrencia 2).
--
-- Antes de guardar el mensaje, Core reserva el client_message_id del autor:
--   INSERT ... IF NOT EXISTS
-- - [applied] = true: escribe el mensaje en messages_by_channel con los
--   valores reservados.
-- - [applied] = false: el envio es un reintento. Core usa el channel_id,
--   bucket y message_id guardados, lee el mensaje y lo confirma. Si no existe
--   (fallo entre la reserva y la escritura), lo escribe con ese message_id; la
--   escritura es idempotente porque la clave es la misma.
--
-- La reserva caduca a los 7 dias: un cliente no reenvia pendientes tan viejos.

CREATE TABLE IF NOT EXISTS sonimbus_core.messages_by_client_id (
    author_id         uuid,
    client_message_id uuid,
    channel_id        uuid,
    bucket            int,
    message_id        timeuuid,
    PRIMARY KEY ((author_id, client_message_id))
) WITH default_time_to_live = 604800;
