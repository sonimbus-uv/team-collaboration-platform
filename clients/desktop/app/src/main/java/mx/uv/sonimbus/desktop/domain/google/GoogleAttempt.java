package mx.uv.sonimbus.desktop.domain.google;

import java.net.URI;
import java.time.Instant;
import java.util.Objects;
import java.util.UUID;

public record GoogleAttempt(
        UUID attemptId, String attemptSecret, URI authorizationUrl, Instant expiresAt) {
    public GoogleAttempt {
        Objects.requireNonNull(attemptId);
        Objects.requireNonNull(attemptSecret);
        Objects.requireNonNull(authorizationUrl);
        Objects.requireNonNull(expiresAt);
    }

    @Override
    public String toString() {
        return "GoogleAttempt[REDACTED]";
    }
}
