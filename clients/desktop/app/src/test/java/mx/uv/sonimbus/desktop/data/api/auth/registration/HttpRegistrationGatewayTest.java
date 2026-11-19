package mx.uv.sonimbus.desktop.data.api.auth.registration;

import static org.junit.jupiter.api.Assertions.assertDoesNotThrow;
import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;
import mx.uv.sonimbus.desktop.domain.registration.PendingRegistration;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationError;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationException;

import org.junit.jupiter.api.Test;

import java.time.Duration;
import java.time.Instant;
import java.util.UUID;

class HttpRegistrationGatewayTest {
    private static final UUID ATTEMPT_ID = UUID.randomUUID();
    private final AuthApi api = mock(AuthApi.class);
    private final HttpRegistrationGateway gateway = new HttpRegistrationGateway(api);

    @Test
    void mapsAttemptIncludingEmailDeliveryFailure() throws Exception {
        Instant expiration = Instant.now().plusSeconds(600);
        when(api.register("ana@uv.mx", "ValidPassword123!"))
                .thenReturn(new RegistrationAttempt(ATTEMPT_ID, expiration.toString(), false));

        PendingRegistration result = gateway.register("ana@uv.mx", "ValidPassword123!");

        assertEquals(ATTEMPT_ID, result.attemptId());
        assertEquals(expiration, result.expiresAt());
        assertFalse(result.emailSent());
    }

    @Test
    void requiresConfirmedVerification() throws Exception {
        when(api.verifyEmail(ATTEMPT_ID, "00123456"))
                .thenReturn(new Account("id", "ana@uv.mx", true, "active"));
        assertDoesNotThrow(() -> gateway.verifyEmail(ATTEMPT_ID, "00123456"));

        when(api.verifyEmail(ATTEMPT_ID, "00123456"))
                .thenReturn(new Account("id", "ana@uv.mx", false, "pending"));
        RegistrationException failure =
                assertThrows(
                        RegistrationException.class,
                        () -> gateway.verifyEmail(ATTEMPT_ID, "00123456"));
        assertEquals(RegistrationError.UNEXPECTED, failure.kind());
        assertNull(failure.getCause());
    }

    @Test
    void rejectsResendForAnotherAttempt() throws Exception {
        when(api.resendVerification(ATTEMPT_ID))
                .thenReturn(
                        new RegistrationAttempt(
                                UUID.randomUUID(),
                                Instant.now().plusSeconds(600).toString(),
                                true));

        assertEquals(
                RegistrationError.UNEXPECTED,
                assertThrows(
                                RegistrationException.class,
                                () -> gateway.resendVerification(ATTEMPT_ID))
                        .kind());
    }

    @Test
    void rejectsMissingResponsesWithoutLeakingDeserializationDetails() {
        RegistrationException failure =
                assertThrows(
                        RegistrationException.class, () -> gateway.register("ana@uv.mx", "secret"));

        assertEquals(RegistrationError.UNEXPECTED, failure.kind());
        assertNull(failure.getCause());
        assertFalse(failure.getMessage().contains("secret"));
        assertThrows(
                RegistrationException.class, () -> gateway.verifyEmail(ATTEMPT_ID, "12345678"));
        assertThrows(RegistrationException.class, () -> gateway.resendVerification(ATTEMPT_ID));
    }

    @Test
    void translatesTransportFailuresForEveryOperation() throws Exception {
        ApiException rateLimit =
                new ApiException(
                        ApiErrorKind.RATE_LIMITED, "register", 429, Duration.ofSeconds(60));
        when(api.register("ana@uv.mx", "secret")).thenThrow(rateLimit);
        when(api.verifyEmail(ATTEMPT_ID, "12345678"))
                .thenThrow(new ApiException(ApiErrorKind.VALIDATION, "verify-email", 400, null));
        when(api.resendVerification(ATTEMPT_ID))
                .thenThrow(new ApiException(ApiErrorKind.SERVER, "resend-verification", 500, null));

        RegistrationException failure =
                assertThrows(
                        RegistrationException.class, () -> gateway.register("ana@uv.mx", "secret"));
        assertEquals(RegistrationError.RATE_LIMITED, failure.kind());
        assertEquals(Duration.ofSeconds(60), failure.retryAfter().orElseThrow());
        assertSame(rateLimit, failure.getCause());
        assertEquals(
                RegistrationError.VALIDATION,
                assertThrows(
                                RegistrationException.class,
                                () -> gateway.verifyEmail(ATTEMPT_ID, "12345678"))
                        .kind());
        assertEquals(
                RegistrationError.SERVER,
                assertThrows(
                                RegistrationException.class,
                                () -> gateway.resendVerification(ATTEMPT_ID))
                        .kind());
    }
}
