package mx.uv.sonimbus.desktop.ui.chat;

import java.util.Objects;
import mx.uv.sonimbus.desktop.data.core.Message;

record ChatEntry(String clientMessageId, String text, Message message, SendState state) {

    ChatEntry {
        Objects.requireNonNull(clientMessageId);
        Objects.requireNonNull(state);
    }

    static ChatEntry pending(String clientMessageId, String text) {
        return new ChatEntry(clientMessageId, text, null, SendState.PENDING);
    }

    static ChatEntry failed(String clientMessageId, String text) {
        return new ChatEntry(clientMessageId, text, null, SendState.FAILED);
    }

    static ChatEntry confirmed(Message message) {
        return new ChatEntry(message.clientMessageId(), message.content(), message, SendState.SENT);
    }

    boolean isPending() {
        return message == null;
    }

    boolean isRetryable() {
        return state == SendState.PENDING && message == null;
    }

    ChatEntry asFailed() {
        return new ChatEntry(clientMessageId, text, null, SendState.FAILED);
    }
}
