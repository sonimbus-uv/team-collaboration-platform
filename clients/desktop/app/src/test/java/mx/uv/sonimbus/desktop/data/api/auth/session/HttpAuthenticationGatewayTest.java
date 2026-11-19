package mx.uv.sonimbus.desktop.data.api.auth.session;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.*;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;

import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationError;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.domain.session.LoginCredentials;

import org.junit.jupiter.api.Test;

import java.time.Clock;
import java.time.Duration;
import java.time.Instant;
import java.time.ZoneOffset;

class HttpAuthenticationGatewayTest {
    private static final Instant NOW = Instant.parse("2026-11-17T00:00:00Z");
    private final AuthApi api = mock(AuthApi.class);
    private final HttpAuthenticationGateway gateway =
            new HttpAuthenticationGateway(api, Clock.fixed(NOW, ZoneOffset.UTC));

    @Test
    void incompleteOrInvalidTokenResponsesAreRejectedWithoutRetainingTheirContent()
            throws Exception {
        for (String body :
                new String[] {
                    "{}",
                    "null",
                    tokenJson().replace("Bearer", "Other"),
                    tokenJson().replace("900", "901"),
                    tokenJson().replace("private-refresh", " "),
                    tokenJson().replace("2030-01-01", "2020-01-01"),
                    tokenJson()
                            .replace(
                                    "\"sid\":\"2f862636-cf2f-4c18-820e-7c29dc449ba0\"",
                                    "\"sid\":null")
                }) {
            TokenResponse response = mapper().readValue(body, TokenResponse.class);
            when(api.login("user@example.invalid", "password")).thenReturn(response);
            AuthenticationException failure =
                    assertThrows(
                            AuthenticationException.class,
                            () ->
                                    gateway.login(
                                            new LoginCredentials(
                                                    "user@example.invalid", "password")));
            assertEquals(AuthenticationError.UNEXPECTED, failure.kind());
            assertNull(failure.getCause());
            assertFalse(failure.toString().contains("private"));
        }
    }

    @Test
    void refreshPreservesOnlyExplicitSafe503AndRateLimitResponses() throws Exception {
        for (int status : new int[] {502, 503, 504}) {
            doThrow(new ApiException(ApiErrorKind.UNAVAILABLE, "refresh", status, null))
                    .when(api)
                    .refresh("refresh");
            AuthenticationException failure =
                    assertThrows(AuthenticationException.class, () -> gateway.refresh("refresh"));
            assertEquals(
                    status == 503 ? AuthenticationError.UNAVAILABLE : AuthenticationError.NETWORK,
                    failure.kind());
        }
        doThrow(new ApiException(ApiErrorKind.RATE_LIMITED, "refresh", 429, Duration.ofSeconds(30)))
                .when(api)
                .refresh("refresh");
        AuthenticationException limited =
                assertThrows(AuthenticationException.class, () -> gateway.refresh("refresh"));
        assertEquals(AuthenticationError.RATE_LIMITED, limited.kind());
        assertEquals(Duration.ofSeconds(30), limited.retryAfter().orElseThrow());
    }

    @Test
    void revokedRefreshIsReportedAsAnExpiredSession() throws Exception {
        when(api.refresh("refresh"))
                .thenThrow(new ApiException(ApiErrorKind.UNEXPECTED, "refresh", 401, null));
        assertEquals(
                AuthenticationError.NO_SESSION,
                assertThrows(AuthenticationException.class, () -> gateway.refresh("refresh"))
                        .kind());
    }

    @Test
    void accessExpiryNeverExceedsTheSessionExpiry() throws Exception {
        String expiry = NOW.plusSeconds(50).toString();
        when(api.login("user@example.invalid", "password"))
                .thenReturn(
                        mapper().readValue(
                                        tokenJson().replace("2030-01-01T00:00:00Z", expiry),
                                        TokenResponse.class));
        var tokens = gateway.login(new LoginCredentials("user@example.invalid", "password"));
        assertEquals(NOW.plusSeconds(50), tokens.access().expiresAt());
    }

    private static ObjectMapper mapper() {
        return new ObjectMapper().setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE);
    }

    private static String tokenJson() {
        return """
               {"access_token":"private-access","refresh_token":"private-refresh",
                "token_type":"Bearer","access_expires_in":900,
                "session_expires_at":"2030-01-01T00:00:00Z",
                "sid":"2f862636-cf2f-4c18-820e-7c29dc449ba0",
                "account_id":"3f862636-cf2f-4c18-820e-7c29dc449ba0"}
               """;
    }
}
