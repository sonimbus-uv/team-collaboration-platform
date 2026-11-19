package mx.uv.sonimbus.desktop.data.api.auth.password;

import mx.uv.sonimbus.desktop.domain.password.RecoveryAttempt;

import java.time.Instant;
import java.util.UUID;

public record RecoveryResponse(UUID recoveryAttemptId, String expiresAt, String message) {
    public RecoveryAttempt toAttempt() {
        return new RecoveryAttempt(recoveryAttemptId, Instant.parse(expiresAt));
    }
}
