package mx.uv.sonimbus.desktop.data.api.auth.password;

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
import mx.uv.sonimbus.desktop.domain.password.ChangePasswordRequest;
import mx.uv.sonimbus.desktop.domain.password.PasswordError;
import mx.uv.sonimbus.desktop.domain.password.PasswordException;
import mx.uv.sonimbus.desktop.domain.password.RecoveryAttempt;
import mx.uv.sonimbus.desktop.domain.password.ResetPasswordRequest;

import org.junit.jupiter.api.Test;

import java.time.Duration;
import java.time.Instant;
import java.util.UUID;

class HttpPasswordGatewayTest {
    private final AuthApi api = mock(AuthApi.class);
    private final HttpPasswordGateway gateway = new HttpPasswordGateway(api);

    @Test
    void mapsRecoveryWithoutExposingServerMessage() throws Exception {
        UUID id = UUID.randomUUID();
        Instant expires = Instant.now().plusSeconds(600);
        when(api.forgotPassword("ana@uv.mx"))
                .thenReturn(new RecoveryResponse(id, expires.toString(), "private detail"));
        assertEquals(new RecoveryAttempt(id, expires), gateway.forgotPassword("ana@uv.mx"));
    }

    @Test
    void rejectsInvalidResponseWithoutLeakingCause() throws Exception {
        when(api.forgotPassword("ana@uv.mx"))
                .thenReturn(new RecoveryResponse(null, "secret-not-a-date", "private"));
        PasswordException failure =
                assertThrows(PasswordException.class, () -> gateway.forgotPassword("ana@uv.mx"));
        assertEquals(PasswordError.UNEXPECTED, failure.kind());
        assertNull(failure.getCause());
        assertFalse(failure.getMessage().contains("secret"));
    }

    @Test
    void mapsKnownProblemTypesAndPreservesRetryAfter() throws Exception {
        String[] problems = {
            "invalid-current-password",
            "invalid-recovery-code",
            "invalid-password",
            "password-unchanged",
            "password-change-not-allowed",
            "credentials-changed"
        };
        PasswordError[] errors = {
            PasswordError.INVALID_CURRENT,
            PasswordError.INVALID_CODE,
            PasswordError.VALIDATION,
            PasswordError.SAME_PASSWORD,
            PasswordError.NOT_ALLOWED,
            PasswordError.CREDENTIALS_CHANGED
        };
        for (int i = 0; i < problems.length; i++) {
            int status = i < 3 ? 400 : 409;
            ApiException failure =
                    new ApiException(
                            new ApiException.ResponseDetails(
                                    status, null, "/problems/" + problems[i]),
                            ApiErrorKind.VALIDATION,
                            "change-password");
            doThrow(failure).when(api).changePassword(any(), any(), any());
            PasswordException translated =
                    assertThrows(
                            PasswordException.class,
                            () ->
                                    gateway.changePassword(
                                            "token",
                                            new ChangePasswordRequest("old", "new", "new")));
            assertEquals(errors[i], translated.kind());
        }
        doThrow(
                        new ApiException(
                                ApiErrorKind.RATE_LIMITED,
                                "change-password",
                                429,
                                Duration.ofSeconds(80)))
                .when(api)
                .changePassword(any(), any(), any());
        PasswordException limited =
                assertThrows(
                        PasswordException.class,
                        () ->
                                gateway.changePassword(
                                        "token", new ChangePasswordRequest("old", "new", "new")));
        assertEquals(Duration.ofSeconds(80), limited.retryAfter().orElseThrow());
    }

    @Test
    void distinguishesUnavailableFromUncertainCommit() throws Exception {
        for (int status : new int[] {500, 502, 504, 503, 401}) {
            doThrow(new ApiException(ApiErrorKind.UNAVAILABLE, "reset-password", status, null))
                    .when(api)
                    .resetPassword(any(), any(), any());
            PasswordError expected =
                    status == 503
                            ? PasswordError.UNAVAILABLE
                            : status == 401 ? PasswordError.NO_SESSION : PasswordError.UNCERTAIN;
            assertEquals(
                    expected,
                    assertThrows(
                                    PasswordException.class,
                                    () ->
                                            gateway.resetPassword(
                                                    new ResetPasswordRequest(
                                                            UUID.randomUUID(),
                                                            "00123456",
                                                            "new",
                                                            "new")))
                            .kind());
        }
    }
}
