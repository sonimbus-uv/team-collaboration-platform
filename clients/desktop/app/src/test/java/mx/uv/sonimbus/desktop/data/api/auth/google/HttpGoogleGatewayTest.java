package mx.uv.sonimbus.desktop.data.api.auth.google;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;
import mx.uv.sonimbus.desktop.domain.google.GoogleError;
import mx.uv.sonimbus.desktop.domain.google.GoogleException;
import mx.uv.sonimbus.desktop.domain.google.GoogleExchange;

import org.junit.jupiter.api.Test;

import java.time.Instant;
import java.util.UUID;

class HttpGoogleGatewayTest {
    private final AuthApi api = mock(AuthApi.class);
    private final HttpGoogleGateway gateway = new HttpGoogleGateway(api);
    private static final String SECRET = "A".repeat(43);

    @Test
    void rejectsUntrustedBrowserSchemesPathsAndMalformedProofs() {
        for (String url :
                new String[] {
                    "javascript:alert(1)",
                    "http://auth.invalid/api/v1/auth/google/authorize?ticket=" + SECRET,
                    "https://auth.invalid/evil?ticket=" + SECRET,
                    "https://auth.invalid/api/v1/auth/google/authorize?ticket="
                            + SECRET
                            + "&extra=value"
                }) {
            assertThrows(
                    RuntimeException.class,
                    () ->
                            new GoogleAttemptResponse(
                                            UUID.randomUUID(),
                                            SECRET,
                                            url,
                                            Instant.now().plusSeconds(600).toString())
                                    .toAttempt());
        }
    }

    @Test
    void mapsAccountLinkConflictAndUncertainExchange() throws Exception {
        when(api.exchangeGoogle(any()))
                .thenThrow(
                        new ApiException(
                                new ApiException.ResponseDetails(
                                        409, null, "/problems/account-link-required"),
                                ApiErrorKind.UNEXPECTED,
                                "google-exchange"));
        GoogleExchange proof = new GoogleExchange(UUID.randomUUID(), SECRET, SECRET, SECRET);
        assertEquals(
                GoogleError.LINK_REQUIRED,
                assertThrows(GoogleException.class, () -> gateway.exchange(proof)).kind());
        doThrow(new ApiException(ApiErrorKind.TIMEOUT, "google-exchange", null))
                .when(api)
                .exchangeGoogle(any());
        assertEquals(
                GoogleError.UNCERTAIN,
                assertThrows(GoogleException.class, () -> gateway.exchange(proof)).kind());
    }

    @Test
    void invalidAttemptResponseDoesNotExposeSecrets() throws Exception {
        when(api.createGoogleAttempt(any())).thenReturn(null);
        GoogleException failure =
                assertThrows(GoogleException.class, () -> gateway.createAttempt(SECRET));
        assertEquals(GoogleError.INVALID_RESPONSE, failure.kind());
        assertNull(failure.getCause());
        assertFalse(failure.getMessage().contains(SECRET));
    }

    @Test
    void recentAuthenticationAndWrongPasswordAreDifferentFromAnExpiredSession() throws Exception {
        String[] problems = {
            "reauthentication-required", "invalid-credentials", "invalid-access-token"
        };
        GoogleError[] expected = {
            GoogleError.REAUTHENTICATION_REQUIRED,
            GoogleError.INVALID_PASSWORD,
            GoogleError.NO_SESSION
        };
        for (int index = 0; index < problems.length; index++) {
            doThrow(
                            new ApiException(
                                    new ApiException.ResponseDetails(
                                            401, null, "/problems/" + problems[index]),
                                    ApiErrorKind.UNEXPECTED,
                                    "google-reauthenticate"))
                    .when(api)
                    .reauthenticateGoogle(any(), any());
            assertEquals(
                    expected[index],
                    assertThrows(
                                    GoogleException.class,
                                    () -> gateway.reauthenticate("session", "password"))
                            .kind());
        }
    }

    @Test
    void connectionResponsePreservesProviderReferenceAndRejectsInvalidShape() {
        var response =
                new GoogleConnectionsResponse(
                        java.util.List.of(
                                new GoogleConnectionsResponse.Connection(
                                        "google",
                                        "reference@gmail.com",
                                        Instant.now().toString())));
        assertEquals("reference@gmail.com", response.toConnection().orElseThrow().providerEmail());
        assertFalse(response.toString().contains("reference"));
        assertThrows(
                RuntimeException.class,
                () ->
                        new GoogleConnectionsResponse(
                                        java.util.List.of(
                                                new GoogleConnectionsResponse.Connection(
                                                        "other", null, Instant.now().toString())))
                                .toConnection());
    }
}
