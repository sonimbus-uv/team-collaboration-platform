package mx.uv.sonimbus.desktop.domain.outbox;

import java.time.Clock;
import java.util.List;
import java.util.Objects;
import java.util.UUID;
import mx.uv.sonimbus.desktop.domain.session.SessionService;

public final class OutboxService {

    private final OutboxStore store;
    private final SessionService sessions;
    private final Clock clock;

    public OutboxService(OutboxStore store, SessionService sessions) {
        this(store, sessions, Clock.systemUTC());
    }

    public OutboxService(OutboxStore store, SessionService sessions, Clock clock) {
        this.store = Objects.requireNonNull(store);
        this.sessions = Objects.requireNonNull(sessions);
        this.clock = Objects.requireNonNull(clock);
    }

    public OutboxMessage enqueue(String channelId, String content) {
        OutboxMessage message = new OutboxMessage(
                UUID.randomUUID().toString(), channelId, content, OutboxStatus.PENDING, clock.instant());
        store.add(account(), message);
        return message;
    }

    public List<OutboxMessage> unsent(String channelId) {
        return store.unsent(account(), channelId);
    }

    public void markFailed(String clientMessageId) {
        store.markFailed(account(), clientMessageId);
    }

    public void confirm(String clientMessageId) {
        store.remove(account(), clientMessageId);
    }

    public void discard(String clientMessageId) {
        store.remove(account(), clientMessageId);
    }

    private String account() {
        return sessions.accountId().orElseThrow(() -> new OutboxException("No active session"));
    }
}
