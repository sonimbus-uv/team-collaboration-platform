package mx.uv.sonimbus.desktop.data.api.auth.session;

import com.fasterxml.jackson.annotation.JsonAutoDetect;

import mx.uv.sonimbus.desktop.domain.session.SessionTokens;

import java.time.Instant;
import java.util.UUID;

@JsonAutoDetect(fieldVisibility = JsonAutoDetect.Visibility.ANY)
public final class TokenResponse {
    private String accessToken;
    private String refreshToken;
    private String tokenType;
    private Long accessExpiresIn;
    private String sessionExpiresAt;
    private UUID sid;
    private UUID accountId;

    public SessionTokens toSessionTokens(Instant requestStarted) {
        if (!"Bearer".equals(tokenType)
                || accessExpiresIn == null
                || accessExpiresIn < 1
                || accessExpiresIn > 900
                || !validCredential(accessToken, 8192)
                || !validCredential(refreshToken, 512)) {
            throw new IllegalArgumentException("Invalid token response");
        }
        Instant sessionExpiry = Instant.parse(sessionExpiresAt);
        if (!sessionExpiry.isAfter(requestStarted)) {
            throw new IllegalArgumentException("Expired token response");
        }
        Instant accessExpiry = requestStarted.plusSeconds(accessExpiresIn);
        if (accessExpiry.isAfter(sessionExpiry)) {
            accessExpiry = sessionExpiry;
        }
        return new SessionTokens(
                new SessionTokens.AccessToken(accessToken, accessExpiry),
                refreshToken,
                new SessionTokens.Identity(sid, accountId),
                sessionExpiry);
    }

    private static boolean validCredential(String value, int maximumLength) {
        return value != null
                && !value.isBlank()
                && value.length() <= maximumLength
                && value.chars()
                        .noneMatch(
                                character ->
                                        Character.isWhitespace(character)
                                                || Character.isISOControl(character));
    }

    @Override
    public String toString() {
        return "TokenResponse[REDACTED]";
    }
}
