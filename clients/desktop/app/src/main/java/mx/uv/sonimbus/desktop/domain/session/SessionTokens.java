package mx.uv.sonimbus.desktop.domain.session;

import java.time.Instant;
import java.util.Objects;
import java.util.UUID;

public record SessionTokens(
        AccessToken access, String refreshToken, Identity identity, Instant expiresAt) {

    public SessionTokens {
        Objects.requireNonNull(access);
        Objects.requireNonNull(identity);
        Objects.requireNonNull(expiresAt);
        if (access.expiresAt().isAfter(expiresAt)) {
            throw new IllegalArgumentException("Access credential exceeds session lifetime");
        }
        if (refreshToken == null || refreshToken.isBlank()) {
            throw new IllegalArgumentException("Missing session credential");
        }
    }

    public record AccessToken(String value, Instant expiresAt) {
        public AccessToken {
            if (value == null || value.isBlank()) {
                throw new IllegalArgumentException("Missing access credential");
            }
            Objects.requireNonNull(expiresAt);
        }

        @Override
        public String toString() {
            return "AccessToken[REDACTED]";
        }
    }

    public record Identity(UUID sessionId, UUID accountId) {
        public Identity {
            Objects.requireNonNull(sessionId);
            Objects.requireNonNull(accountId);
        }
    }

    @Override
    public String toString() {
        return "SessionTokens[REDACTED]";
    }
}
