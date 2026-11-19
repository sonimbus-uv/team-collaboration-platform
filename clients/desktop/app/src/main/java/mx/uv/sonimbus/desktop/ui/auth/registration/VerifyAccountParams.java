package mx.uv.sonimbus.desktop.ui.auth.registration;

import mx.uv.sonimbus.desktop.domain.registration.PendingRegistration;

import java.time.Instant;
import java.util.Objects;
import java.util.UUID;

public record VerifyAccountParams(
        String email, UUID attemptId, Instant expiresAt, boolean emailSent) {

    public VerifyAccountParams {
        Objects.requireNonNull(email, "email");
        Objects.requireNonNull(attemptId, "attemptId");
        Objects.requireNonNull(expiresAt, "expiresAt");
    }

    public static VerifyAccountParams afterRegistration(String email, PendingRegistration attempt) {
        return new VerifyAccountParams(
                email, attempt.attemptId(), attempt.expiresAt(), attempt.emailSent());
    }
}
