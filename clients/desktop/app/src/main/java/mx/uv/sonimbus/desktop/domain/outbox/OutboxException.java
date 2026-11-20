package mx.uv.sonimbus.desktop.domain.outbox;

public final class OutboxException extends RuntimeException {

    public OutboxException(String message) {
        super(message);
    }

    public OutboxException(String message, Throwable cause) {
        super(message, cause);
    }
}
