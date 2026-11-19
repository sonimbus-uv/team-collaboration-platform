package mx.uv.sonimbus.desktop.domain.password;

import java.util.UUID;

public record ResetPasswordRequest(
        UUID recoveryAttemptId, String code, String newPassword, String confirmation) {
    @Override
    public String toString() {
        return "ResetPasswordRequest[REDACTED]";
    }
}
