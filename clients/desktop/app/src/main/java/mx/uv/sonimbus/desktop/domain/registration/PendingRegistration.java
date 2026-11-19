package mx.uv.sonimbus.desktop.domain.registration;

import java.time.Instant;
import java.util.Objects;
import java.util.UUID;

public record PendingRegistration(UUID attemptId, Instant expiresAt, boolean emailSent) {
    public PendingRegistration {
        Objects.requireNonNull(attemptId);
        Objects.requireNonNull(expiresAt);
    }
}
