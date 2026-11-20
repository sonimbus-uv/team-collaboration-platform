package mx.uv.sonimbus.desktop.domain.outbox;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;

import java.time.Clock;
import java.time.Instant;
import java.time.ZoneOffset;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationGateway;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.domain.session.SessionTokens;
import org.junit.jupiter.api.Test;

class OutboxServiceTest {

    private static final class MemoryStore implements OutboxStore {
        final List<String> accounts = new ArrayList<>();
        final List<OutboxMessage> rows = new ArrayList<>();

        @Override
        public void add(String accountId, OutboxMessage message) {
            accounts.add(accountId);
            rows.add(message);
        }

        @Override
        public List<OutboxMessage> unsent(String accountId, String channelId) {
            return rows.stream().filter(row -> row.channelId().equals(channelId)).toList();
        }

        @Override
        public void markFailed(String accountId, String clientMessageId) {
            rows.replaceAll(row -> row.clientMessageId().equals(clientMessageId)
                    ? new OutboxMessage(row.clientMessageId(), row.channelId(), row.content(),
                            OutboxStatus.FAILED, row.createdAt())
                    : row);
        }

        @Override
        public void remove(String accountId, String clientMessageId) {
            rows.removeIf(row -> row.clientMessageId().equals(clientMessageId));
        }
    }

    private final Clock clock = Clock.fixed(Instant.parse("2026-11-20T10:00:00Z"), ZoneOffset.UTC);
    private final MemoryStore store = new MemoryStore();
    private final UUID account = UUID.randomUUID();

    private SessionService signedIn() throws AuthenticationException {
        SessionService sessions = new SessionService(mock(AuthenticationGateway.class), clock);
        sessions.signInFederated(new SessionTokens(
                new SessionTokens.AccessToken("access", clock.instant().plusSeconds(900)),
                "refresh", new SessionTokens.Identity(UUID.randomUUID(), account),
                clock.instant().plusSeconds(3600)));
        return sessions;
    }

    @Test
    void enqueuedMessageGetsAFreshClientMessageIdAndIsStoredForTheAccount() throws Exception {
        OutboxService outbox = new OutboxService(store, signedIn(), clock);

        OutboxMessage first = outbox.enqueue("c1", "hola");
        OutboxMessage second = outbox.enqueue("c1", "hola");

        assertNotEquals(first.clientMessageId(), second.clientMessageId());
        assertEquals(OutboxStatus.PENDING, first.status());
        assertEquals(List.of(account.toString(), account.toString()), store.accounts);
    }

    @Test
    void confirmedMessageLeavesTheOutbox() throws Exception {
        OutboxService outbox = new OutboxService(store, signedIn(), clock);
        OutboxMessage queued = outbox.enqueue("c1", "hola");

        outbox.confirm(queued.clientMessageId());

        assertTrue(outbox.unsent("c1").isEmpty());
    }

    @Test
    void rejectedMessageIsKeptAsFailedUntilDiscarded() throws Exception {
        OutboxService outbox = new OutboxService(store, signedIn(), clock);
        OutboxMessage queued = outbox.enqueue("c1", "hola");

        outbox.markFailed(queued.clientMessageId());

        assertEquals(OutboxStatus.FAILED, outbox.unsent("c1").get(0).status());
        outbox.discard(queued.clientMessageId());
        assertTrue(outbox.unsent("c1").isEmpty());
    }

    @Test
    void withoutASessionNothingIsQueued() {
        OutboxService outbox = new OutboxService(store,
                new SessionService(mock(AuthenticationGateway.class), clock), clock);

        assertThrows(OutboxException.class, () -> outbox.enqueue("c1", "hola"));
        assertTrue(store.rows.isEmpty());
    }
}
