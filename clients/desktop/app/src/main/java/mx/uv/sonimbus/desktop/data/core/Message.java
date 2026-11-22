package mx.uv.sonimbus.desktop.data.core;

import java.time.Instant;

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

    public record Author(String id, String displayName) {
    }

    public Instant createdInstant() {
        return Instant.parse(createdAt);
    }
}
