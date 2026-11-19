package mx.uv.sonimbus.desktop.data.api.auth.google;

import mx.uv.sonimbus.desktop.domain.google.GoogleAttempt;

import java.net.URI;
import java.time.Instant;
import java.util.UUID;

public record GoogleAttemptResponse(
        UUID attemptId, String attemptSecret, String authorizationUrl, String expiresAt) {
    public GoogleAttempt toAttempt() {
        URI url = URI.create(authorizationUrl);
        if (!attemptSecret.matches("[A-Za-z0-9_-]{43}")
                || !"https".equals(url.getScheme())
                || url.getHost() == null
                || url.getRawUserInfo() != null
                || url.getRawFragment() != null
                || !"/api/v1/auth/google/authorize".equals(url.getRawPath())
                || url.getRawQuery() == null
                || !url.getRawQuery().matches("ticket=[A-Za-z0-9_-]{43}")) {
            throw new IllegalArgumentException("Invalid Google attempt response");
        }
        return new GoogleAttempt(attemptId, attemptSecret, url, Instant.parse(expiresAt));
    }

    @Override
    public String toString() {
        return "GoogleAttemptResponse[REDACTED]";
    }
}
