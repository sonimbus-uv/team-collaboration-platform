package mx.uv.sonimbus.desktop.domain.outbox;

import java.time.Instant;
import java.util.Objects;

public record OutboxMessage(
        String clientMessageId,
        String channelId,
        String content,
        OutboxStatus status,
        Instant createdAt) {

    public OutboxMessage {
        Objects.requireNonNull(clientMessageId);
        Objects.requireNonNull(channelId);
        Objects.requireNonNull(content);
        Objects.requireNonNull(status);
        Objects.requireNonNull(createdAt);
    }
}
