package mx.uv.sonimbus.desktop.data.core;

import java.time.Instant;

/**
 * Mensaje de un canal de texto. {@code id} es el timeuuid que asigna Core y define el orden
 * (D-08); con {@code deleted}, {@code content} es nulo.
 */
public record Message(
        String id,
        String channelId,
        String authorId,
        Author author,
        String clientMessageId,
        String content,
        String replyTo,
        String fileId,
        String createdAt,
        String editedAt,
        boolean deleted) {

    /** Nombre visible del autor al enviar; nulo sin perfil o con la cuenta eliminada. */
    public record Author(String id, String displayName) {
    }

    public Instant createdInstant() {
        return Instant.parse(createdAt);
    }
}
