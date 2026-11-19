package mx.uv.sonimbus.desktop.domain.password;

import java.time.Instant;
import java.util.Objects;
import java.util.UUID;

public record RecoveryAttempt(UUID attemptId, Instant expiresAt) {
    public RecoveryAttempt {
        Objects.requireNonNull(attemptId);
        Objects.requireNonNull(expiresAt);
    }
}
