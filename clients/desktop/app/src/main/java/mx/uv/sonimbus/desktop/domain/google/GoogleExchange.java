package mx.uv.sonimbus.desktop.domain.google;

import java.util.UUID;

public record GoogleExchange(
        UUID attemptId, String attemptSecret, String completionCode, String codeVerifier) {
    @Override
    public String toString() {
        return "GoogleExchange[REDACTED]";
    }
}
