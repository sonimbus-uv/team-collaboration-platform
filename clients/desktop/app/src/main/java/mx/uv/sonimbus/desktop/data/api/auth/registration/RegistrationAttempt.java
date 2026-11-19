package mx.uv.sonimbus.desktop.data.api.auth.registration;

import java.time.Instant;
import java.util.Objects;
import java.util.UUID;

public record RegistrationAttempt(
        UUID registrationAttemptId, String expiresAt, boolean verificationEmailSent) {

    public RegistrationAttempt {
        Objects.requireNonNull(registrationAttemptId, "registrationAttemptId");
        Instant.parse(expiresAt);
    }

    public Instant expirationTime() {
        return Instant.parse(expiresAt);
    }
}
