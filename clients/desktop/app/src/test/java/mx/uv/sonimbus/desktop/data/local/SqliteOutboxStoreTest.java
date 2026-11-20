package mx.uv.sonimbus.desktop.data.local;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.nio.file.Path;
import java.time.Instant;
import java.util.List;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxException;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxMessage;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxStatus;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

class SqliteOutboxStoreTest {

    @TempDir
    Path directory;

    private static OutboxMessage message(String id, String channel, String content) {
        return new OutboxMessage(id, channel, content, OutboxStatus.PENDING, Instant.parse("2026-11-20T10:00:00Z"));
    }

    @Test
    void unsentMessagesSurviveReopeningTheDatabase() {
        Path file = directory.resolve("outbox.db");
        new SqliteOutboxStore(file).add("a1", message("k1", "c1", "hola"));

        List<OutboxMessage> unsent = new SqliteOutboxStore(file).unsent("a1", "c1");

        assertEquals(List.of(message("k1", "c1", "hola")), unsent);
    }

    @Test
    void messagesComeBackInTheOrderTheyWereQueued() {
        SqliteOutboxStore store = new SqliteOutboxStore(directory.resolve("outbox.db"));
        store.add("a1", message("k2", "c1", "segundo"));
        store.add("a1", message("k1", "c1", "primero"));

        assertEquals(List.of("k2", "k1"), store.unsent("a1", "c1").stream().map(OutboxMessage::clientMessageId).toList());
    }

    @Test
    void messagesAreScopedByAccountAndChannel() {
        SqliteOutboxStore store = new SqliteOutboxStore(directory.resolve("outbox.db"));
        store.add("a1", message("k1", "c1", "uno"));
        store.add("a1", message("k2", "c2", "dos"));
        store.add("a2", message("k3", "c1", "tres"));

        assertEquals(1, store.unsent("a1", "c1").size());
        assertEquals("k3", store.unsent("a2", "c1").get(0).clientMessageId());
    }

    @Test
    void markedFailedMessageKeepsItsContentAndStatus() {
        SqliteOutboxStore store = new SqliteOutboxStore(directory.resolve("outbox.db"));
        store.add("a1", message("k1", "c1", "hola"));

        store.markFailed("a1", "k1");

        OutboxMessage stored = store.unsent("a1", "c1").get(0);
        assertEquals(OutboxStatus.FAILED, stored.status());
        assertEquals("hola", stored.content());
    }

    @Test
    void removedMessageIsNotReturnedAgain() {
        SqliteOutboxStore store = new SqliteOutboxStore(directory.resolve("outbox.db"));
        store.add("a1", message("k1", "c1", "hola"));

        store.remove("a1", "k1");

        assertTrue(store.unsent("a1", "c1").isEmpty());
    }

    @Test
    void sameClientMessageIdCannotBeQueuedTwice() {
        SqliteOutboxStore store = new SqliteOutboxStore(directory.resolve("outbox.db"));
        store.add("a1", message("k1", "c1", "hola"));

        assertThrows(OutboxException.class, () -> store.add("a1", message("k1", "c1", "hola")));
    }
}
