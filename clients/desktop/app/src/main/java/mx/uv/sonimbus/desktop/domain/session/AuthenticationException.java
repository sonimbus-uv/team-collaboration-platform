package mx.uv.sonimbus.desktop.domain.session;

import java.time.Duration;
import java.util.Optional;

public final class AuthenticationException extends Exception {
    private final AuthenticationError kind;
    private final Duration retryAfter;

    public AuthenticationException(AuthenticationError kind) {
        this(kind, null, null);
    }

    public AuthenticationException(AuthenticationError kind, Duration retryAfter, Throwable cause) {
        super("Authentication operation failed: " + kind, cause);
        this.kind = kind;
        this.retryAfter = retryAfter;
    }

    public AuthenticationError kind() {
        return kind;
    }

    public Optional<Duration> retryAfter() {
        return Optional.ofNullable(retryAfter);
    }
}
