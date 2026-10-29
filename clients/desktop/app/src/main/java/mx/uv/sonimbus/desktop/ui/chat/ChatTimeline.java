package mx.uv.sonimbus.desktop.ui.chat;

import java.time.Instant;
import java.util.ArrayList;
import java.util.Collection;
import java.util.Collections;
import java.util.List;
import java.util.Objects;
import mx.uv.sonimbus.desktop.data.core.Message;

/**
 * Orden del historial sin JavaFX, para probarlo aparte: mensajes confirmados por fecha del
 * timeuuid (D-08) y, al final, los propios pendientes en el orden en que se escribieron. Un
 * mensaje que llega por la respuesta y por la difusión se guarda una sola vez.
 */
final class ChatTimeline {

    private final List<ChatEntry> entries;

    ChatTimeline(List<ChatEntry> entries) {
        this.entries = Objects.requireNonNull(entries);
    }

    /** Agrega o confirma; devuelve {@code false} si ya estaba. */
    boolean accept(Message message) {
        for (int i = 0; i < entries.size(); i++) {
            ChatEntry entry = entries.get(i);
            if (!entry.isPending() && entry.message().id().equals(message.id())) {
                return false;
            }
            if (entry.isPending() && entry.clientMessageId().equals(message.clientMessageId())) {
                entries.remove(i);
                break;
            }
        }
        entries.add(insertionIndex(message.createdInstant()), ChatEntry.confirmed(message));
        return true;
    }

    void acceptAll(Collection<Message> messages) {
        for (Message message : messages) {
            accept(message);
        }
    }

    void addPending(ChatEntry entry) {
        entries.add(entry);
    }

    /** Marca el pendiente como no enviado; se reintenta al reconectar. */
    void markFailed(String clientMessageId) {
        replacePending(clientMessageId, true);
    }

    void markRetrying(String clientMessageId) {
        replacePending(clientMessageId, false);
    }

    void removePending(String clientMessageId) {
        entries.removeIf(entry -> entry.isPending() && entry.clientMessageId().equals(clientMessageId));
    }

    List<ChatEntry> pending() {
        return entries.stream().filter(ChatEntry::isPending).toList();
    }

    private void replacePending(String clientMessageId, boolean failed) {
        for (int i = 0; i < entries.size(); i++) {
            ChatEntry entry = entries.get(i);
            if (entry.isPending() && entry.clientMessageId().equals(clientMessageId)) {
                entries.set(i, failed ? entry.asFailed() : entry.asRetrying());
                return;
            }
        }
    }

    /** Después del último confirmado anterior o igual; los pendientes quedan al final. */
    private int insertionIndex(Instant createdAt) {
        int index = 0;
        for (int i = 0; i < entries.size(); i++) {
            ChatEntry entry = entries.get(i);
            if (entry.isPending()) {
                break;
            }
            if (!entry.message().createdInstant().isAfter(createdAt)) {
                index = i + 1;
            }
        }
        return index;
    }

    static List<Message> oldestFirst(List<Message> newestFirst) {
        List<Message> copy = new ArrayList<>(newestFirst);
        Collections.reverse(copy);
        return copy;
    }
}
