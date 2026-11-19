package mx.uv.sonimbus.desktop.ui.chat;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.ArrayList;
import java.util.List;
import mx.uv.sonimbus.desktop.data.core.Message;
import org.junit.jupiter.api.Test;

class ChatTimelineTest {

    private final List<ChatEntry> entries = new ArrayList<>();
    private final ChatTimeline timeline = new ChatTimeline(entries);

    private static Message message(String id, String clientId, String at) {
        return new Message(id, "c1", "u1", new Message.Author("u1", "Ana"), clientId, "texto " + id,
                null, null, "2026-10-29T15:00:" + at + "Z", null, false);
    }

    private List<String> ids() {
        return entries.stream().map(e -> e.isPending() ? "pending:" + e.clientMessageId() : e.message().id()).toList();
    }

    @Test
    void sameMessageFromReplyAndBroadcastIsKeptOnce() {
        assertTrue(timeline.accept(message("m1", "k1", "01")));
        assertFalse(timeline.accept(message("m1", "k1", "01")));
        assertEquals(List.of("m1"), ids());
    }

    @Test
    void confirmedMessageReplacesItsPendingEntry() {
        timeline.addPending(ChatEntry.pending("k1", "hola"));

        timeline.accept(message("m1", "k1", "01"));

        assertEquals(List.of("m1"), ids());
    }

    @Test
    void messagesAreOrderedByServerTimeAndPendingStayLast() {
        timeline.accept(message("m2", "k2", "02"));
        timeline.addPending(ChatEntry.pending("k9", "mío"));
        timeline.accept(message("m1", "k1", "01"));
        timeline.accept(message("m3", "k3", "03"));

        assertEquals(List.of("m1", "m2", "m3", "pending:k9"), ids());
    }

    @Test
    void historyNewestFirstIsShownOldestFirst() {
        timeline.acceptAll(ChatTimeline.oldestFirst(List.of(message("m2", "k2", "02"), message("m1", "k1", "01"))));

        assertEquals(List.of("m1", "m2"), ids());
    }

    @Test
    void failedPendingKeepsItsClientIdForTheRetry() {
        timeline.addPending(ChatEntry.pending("k1", "hola"));

        timeline.markFailed("k1");

        assertTrue(entries.get(0).failed());
        assertEquals("k1", timeline.pending().get(0).clientMessageId());
        timeline.markRetrying("k1");
        assertFalse(entries.get(0).failed());
        timeline.removePending("k1");
        assertTrue(entries.isEmpty());
    }
}
