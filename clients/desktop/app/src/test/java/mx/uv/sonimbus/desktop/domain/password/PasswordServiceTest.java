package mx.uv.sonimbus.desktop.domain.password;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.verifyNoInteractions;
import static org.mockito.Mockito.when;

import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationGateway;
import mx.uv.sonimbus.desktop.domain.session.LoginCredentials;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.domain.session.SessionTokens;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.time.Clock;
import java.time.Duration;
import java.time.Instant;
import java.time.ZoneOffset;
import java.util.Set;
import java.util.UUID;

class PasswordServiceTest {
    private static final Instant NOW = Instant.parse("2026-11-18T12:00:00Z");
    private static final UUID ATTEMPT = UUID.randomUUID();
    private static final String NEW_PASSWORD = " NewPassword123! ";
    private final Clock clock = Clock.fixed(NOW, ZoneOffset.UTC);
    private final AuthenticationGateway authentication = mock(AuthenticationGateway.class);
    private final PasswordGateway gateway = mock(PasswordGateway.class);
    private final SessionService sessions = new SessionService(authentication, clock);
    private final PasswordService passwords = new PasswordService(gateway, sessions, clock);

    @BeforeEach
    void signIn() throws Exception {
        SessionTokens tokens =
                new SessionTokens(
                        new SessionTokens.AccessToken("access-token", NOW.plusSeconds(900)),
                        "refresh-token",
                        new SessionTokens.Identity(UUID.randomUUID(), UUID.randomUUID()),
                        NOW.plusSeconds(3600));
        when(authentication.login(any())).thenReturn(tokens);
        sessions.login(new LoginCredentials("ana@uv.mx", "old"));
    }

    @Test
    void recoveryNormalizesEmailAndPreservesOpaqueAttempt() throws Exception {
        RecoveryAttempt accepted = new RecoveryAttempt(ATTEMPT, NOW.plusSeconds(600));
        when(gateway.forgotPassword("ana@uv.mx")).thenReturn(accepted);
        assertSame(accepted, passwords.forgotPassword(" ANA@UV.MX "));
        assertTrue(sessions.isAuthenticated());
    }

    @Test
    void rejectsInvalidEmailWithoutSendingRequest() {
        for (String email : new String[] {null, "", "bad"}) {
            assertThrows(PasswordException.class, () -> passwords.forgotPassword(email));
        }
        verifyNoInteractions(gateway);
    }

    @Test
    void rejectsExpiredRecoveryResponse() throws Exception {
        when(gateway.forgotPassword("ana@uv.mx")).thenReturn(new RecoveryAttempt(ATTEMPT, NOW));
        assertEquals(
                PasswordError.UNEXPECTED,
                assertThrows(PasswordException.class, () -> passwords.forgotPassword("ana@uv.mx"))
                        .kind());
    }

    @Test
    void resetPreservesCodeAndPasswordAndClearsSession() throws Exception {
        ResetPasswordRequest request = reset("00123456");
        passwords.resetPassword(request);
        verify(gateway).resetPassword(request);
        assertFalse(sessions.isAuthenticated());
        assertThrows(AuthenticationException.class, sessions::accessToken);
        verify(authentication, never()).refresh(any());
    }

    @Test
    void validatesResetBeforeCallingGateway() {
        for (String code : new String[] {null, "1234567", "123456789", "１２３４５６７８"}) {
            assertEquals(
                    PasswordError.INVALID_CODE,
                    assertThrows(
                                    PasswordException.class,
                                    () -> passwords.resetPassword(reset(code)))
                            .kind());
        }
        PasswordException invalid =
                assertThrows(
                        PasswordException.class,
                        () ->
                                passwords.resetPassword(
                                        new ResetPasswordRequest(
                                                ATTEMPT, "00123456", "weak", "other")));
        assertFalse(invalid.problems().isEmpty());
        verifyNoInteractions(gateway);
    }

    @Test
    void changeAcceptsLegacyCurrentPasswordAndRequiresLoginAfterSuccess() throws Exception {
        ChangePasswordRequest request =
                new ChangePasswordRequest("old", NEW_PASSWORD, NEW_PASSWORD);
        passwords.changePassword(request);
        verify(gateway).changePassword("access-token", request);
        assertFalse(sessions.isAuthenticated());
    }

    @Test
    void rejectsMissingCurrentPasswordAndUnchangedPasswordLocally() {
        assertEquals(
                PasswordError.INVALID_CURRENT,
                assertThrows(
                                PasswordException.class,
                                () ->
                                        passwords.changePassword(
                                                new ChangePasswordRequest(
                                                        "", NEW_PASSWORD, NEW_PASSWORD)))
                        .kind());
        assertEquals(
                PasswordError.SAME_PASSWORD,
                assertThrows(
                                PasswordException.class,
                                () ->
                                        passwords.changePassword(
                                                new ChangePasswordRequest(
                                                        NEW_PASSWORD, NEW_PASSWORD, NEW_PASSWORD)))
                        .kind());
        verifyNoInteractions(gateway);
    }

    @Test
    void incorrectCurrentPasswordPreservesSession() throws Exception {
        doThrow(new PasswordException(PasswordError.INVALID_CURRENT))
                .when(gateway)
                .changePassword(any(), any());
        assertEquals(
                PasswordError.INVALID_CURRENT,
                assertThrows(PasswordException.class, () -> passwords.changePassword(change()))
                        .kind());
        assertTrue(sessions.isAuthenticated());
    }

    @Test
    void unavailableAndRateLimitPreserveSessionAndRetryAfter() throws Exception {
        PasswordException limited =
                new PasswordException(
                        PasswordError.RATE_LIMITED, Duration.ofSeconds(90), Set.of(), null);
        doThrow(limited).when(gateway).changePassword(any(), any());
        assertSame(
                limited,
                assertThrows(PasswordException.class, () -> passwords.changePassword(change())));
        assertTrue(sessions.isAuthenticated());
        doThrow(new PasswordException(PasswordError.UNAVAILABLE))
                .when(gateway)
                .changePassword(any(), any());
        assertThrows(PasswordException.class, () -> passwords.changePassword(change()));
        assertTrue(sessions.isAuthenticated());
    }

    @Test
    void uncertainMutationClearsSessionAndIsNeverRetried() throws Exception {
        doThrow(new PasswordException(PasswordError.TIMEOUT))
                .when(gateway)
                .changePassword(any(), any());
        assertEquals(
                PasswordError.UNCERTAIN,
                assertThrows(PasswordException.class, () -> passwords.changePassword(change()))
                        .kind());
        assertFalse(sessions.isAuthenticated());
        verify(gateway, times(1)).changePassword(any(), any());
    }

    @Test
    void resetFailurePreservesSessionUnlessResultIsUncertain() throws Exception {
        doThrow(new PasswordException(PasswordError.INVALID_CODE))
                .when(gateway)
                .resetPassword(any());
        assertThrows(PasswordException.class, () -> passwords.resetPassword(reset("00123456")));
        assertTrue(sessions.isAuthenticated());
        doThrow(new PasswordException(PasswordError.UNCERTAIN)).when(gateway).resetPassword(any());
        assertEquals(
                PasswordError.UNCERTAIN,
                assertThrows(
                                PasswordException.class,
                                () -> passwords.resetPassword(reset("00123456")))
                        .kind());
        assertFalse(sessions.isAuthenticated());
    }

    @Test
    void unauthorizedChangeClearsSession() throws Exception {
        doThrow(new PasswordException(PasswordError.NO_SESSION))
                .when(gateway)
                .changePassword(any(), any());
        assertThrows(PasswordException.class, () -> passwords.changePassword(change()));
        assertFalse(sessions.isAuthenticated());
    }

    @Test
    void requestDescriptionsNeverExposeSecrets() {
        assertFalse(change().toString().contains(NEW_PASSWORD));
        assertFalse(reset("00123456").toString().contains("00123456"));
        assertFalse(reset("00123456").toString().contains(NEW_PASSWORD));
    }

    private static ResetPasswordRequest reset(String code) {
        return new ResetPasswordRequest(ATTEMPT, code, NEW_PASSWORD, NEW_PASSWORD);
    }

    private static ChangePasswordRequest change() {
        return new ChangePasswordRequest("old", NEW_PASSWORD, NEW_PASSWORD);
    }
}
