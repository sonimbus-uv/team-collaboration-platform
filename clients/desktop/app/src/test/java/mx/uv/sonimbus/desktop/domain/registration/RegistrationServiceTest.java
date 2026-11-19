package mx.uv.sonimbus.desktop.domain.registration;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.verifyNoInteractions;
import static org.mockito.Mockito.when;

import org.junit.jupiter.api.Test;

import java.time.Duration;
import java.time.Instant;
import java.util.Set;
import java.util.UUID;

class RegistrationServiceTest {
    private static final UUID ATTEMPT_ID = UUID.randomUUID();
    private static final String PASSWORD = " ValidPassword123! ";
    private final RegistrationGateway gateway = mock(RegistrationGateway.class);
    private final RegistrationService registrations = new RegistrationService(gateway);

    @Test
    void normalizesEmailWithoutChangingPassword() throws Exception {
        PendingRegistration pending =
                new PendingRegistration(ATTEMPT_ID, Instant.now().plusSeconds(600), false);
        when(gateway.register("ana@uv.mx", PASSWORD)).thenReturn(pending);

        assertSame(
                pending,
                registrations.register(new RegistrationRequest(" ANA@UV.MX ", PASSWORD, PASSWORD)));
        verify(gateway).register("ana@uv.mx", PASSWORD);
    }

    @Test
    void rejectsInvalidRegistrationBeforeCallingGateway() {
        RegistrationException failure =
                assertThrows(
                        RegistrationException.class,
                        () ->
                                registrations.register(
                                        new RegistrationRequest("bad", "short", "other")));

        assertEquals(RegistrationError.VALIDATION, failure.kind());
        assertTrue(failure.problems().contains(RegistrationValidator.Problem.EMAIL_INVALID));
        assertTrue(
                failure.problems().contains(RegistrationValidator.Problem.CONFIRMATION_MISMATCH));
        assertThrows(UnsupportedOperationException.class, () -> failure.problems().clear());
        assertThrows(RegistrationException.class, () -> registrations.register(null));
        verifyNoInteractions(gateway);
    }

    @Test
    void verifiesCodeWithoutRemovingLeadingZeros() throws Exception {
        registrations.verifyEmail(ATTEMPT_ID, "00123456");
        verify(gateway).verifyEmail(ATTEMPT_ID, "00123456");
    }

    @Test
    void rejectsInvalidCodesAndMissingAttemptBeforeCallingGateway() {
        for (String code :
                new String[] {null, "", "1234567", "123456789", "1234567a", "１２３４５６７８"}) {
            assertThrows(
                    RegistrationException.class, () -> registrations.verifyEmail(ATTEMPT_ID, code));
        }
        assertThrows(
                RegistrationException.class, () -> registrations.verifyEmail(null, "12345678"));
        assertThrows(RegistrationException.class, () -> registrations.resendVerification(null));
        verifyNoInteractions(gateway);
    }

    @Test
    void resendsWithoutChangingOriginalExpiration() throws Exception {
        PendingRegistration pending =
                new PendingRegistration(ATTEMPT_ID, Instant.now().plusSeconds(120), true);
        when(gateway.resendVerification(ATTEMPT_ID)).thenReturn(pending);

        assertSame(pending, registrations.resendVerification(ATTEMPT_ID));
        verify(gateway).resendVerification(ATTEMPT_ID);
    }

    @Test
    void propagatesFailureAndRetryAfterWithoutRetrying() throws Exception {
        RegistrationException unavailable =
                new RegistrationException(
                        RegistrationError.RATE_LIMITED, Duration.ofSeconds(90), Set.of(), null);
        when(gateway.resendVerification(ATTEMPT_ID)).thenThrow(unavailable);

        assertSame(
                unavailable,
                assertThrows(
                        RegistrationException.class,
                        () -> registrations.resendVerification(ATTEMPT_ID)));
        verify(gateway, times(1)).resendVerification(ATTEMPT_ID);
    }

    @Test
    void requestDoesNotExposeCredentials() {
        String description = new RegistrationRequest("ana@uv.mx", PASSWORD, PASSWORD).toString();
        assertFalse(description.contains(PASSWORD));
        assertFalse(description.contains("ana@uv.mx"));
    }
}
