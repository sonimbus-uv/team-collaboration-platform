package mx.uv.sonimbus.desktop.domain.google;

import java.time.Duration;
import java.util.Optional;

public final class GoogleException extends Exception {
    private final GoogleError kind;
    private final Duration retryAfter;

    public GoogleException(GoogleError kind) {
        this(kind, null, null);
    }

    public GoogleException(GoogleError kind, Duration retryAfter, Throwable cause) {
        super("Google authentication failed: " + kind, cause);
        this.kind = kind;
        this.retryAfter = retryAfter;
    }

    public GoogleError kind() {
        return kind;
    }

    public Optional<Duration> retryAfter() {
        return Optional.ofNullable(retryAfter);
    }
}
