package mx.uv.sonimbus.desktop.ui.chat;

import java.util.Objects;
import mx.uv.sonimbus.desktop.data.core.Message;

/**
 * Fila del historial: un mensaje confirmado por Core o uno propio que todavía no llega
 * ({@code message} nulo). {@code clientMessageId} une ambos estados y se reutiliza en cada
 * reintento (RNF-CON-01).
 */
record ChatEntry(String clientMessageId, String text, Message message, boolean failed) {

    ChatEntry {
        Objects.requireNonNull(clientMessageId);
    }

    static ChatEntry pending(String clientMessageId, String text) {
        return new ChatEntry(clientMessageId, text, null, false);
    }

    static ChatEntry confirmed(Message message) {
        return new ChatEntry(message.clientMessageId(), message.content(), message, false);
    }

    boolean isPending() {
        return message == null;
    }

    ChatEntry asFailed() {
        return new ChatEntry(clientMessageId, text, null, true);
    }

    ChatEntry asRetrying() {
        return new ChatEntry(clientMessageId, text, null, false);
    }
}
