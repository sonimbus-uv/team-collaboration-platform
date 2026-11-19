package mx.uv.sonimbus.desktop.domain.session;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.*;

import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

import java.time.Clock;
import java.time.Duration;
import java.time.Instant;
import java.util.UUID;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;

class SessionServiceTest {
    private static final Instant NOW = Instant.parse("2026-11-17T00:00:00Z");
    private static final SessionTokens.Identity IDENTITY =
            new SessionTokens.Identity(UUID.randomUUID(), UUID.randomUUID());
    private final AuthenticationGateway gateway = mock(AuthenticationGateway.class);
    private final AtomicReference<Instant> time = new AtomicReference<>(NOW);
    private SessionService sessions;

    @BeforeEach
    void createService() {
        Clock clock = mock(Clock.class);
        when(clock.instant()).thenAnswer(invocation -> time.get());
        sessions = new SessionService(gateway, clock);
    }

    @Test
    void loginNormalizesOnlyTheEmailAndAcceptsLegacyPasswordComposition() throws Exception {
        when(gateway.login(any())).thenReturn(tokens("initial", 900));
        sessions.login(new LoginCredentials("  USER@EXAMPLE.INVALID ", " legacy "));
        verify(gateway).login(new LoginCredentials("user@example.invalid", " legacy "));
        assertEquals("user@example.invalid", sessions.email().orElseThrow());
        assertTrue(sessions.isAuthenticated());
    }

    @Test
    void invalidInputNeverReachesTheGateway() {
        assertThrows(
                AuthenticationException.class,
                () -> sessions.login(new LoginCredentials("invalid", "password")));
        assertThrows(
                AuthenticationException.class,
                () -> sessions.login(new LoginCredentials("user@example.invalid", "")));
        verifyNoInteractions(gateway);
    }

    @Test
    void confirmedLogoutClearsTokensAndAnEmptyLogoutIsIdempotent() throws Exception {
        signIn(900);
        sessions.logout();
        sessions.logout();
        verify(gateway, times(1)).logout("refresh-initial");
        assertFalse(sessions.isAuthenticated());
        assertTrue(sessions.email().isEmpty());
        assertThrows(AuthenticationException.class, sessions::accessToken);
    }

    @Test
    void failedLogoutKeepsCredentialsForAnIdempotentRetry() throws Exception {
        signIn(900);
        doThrow(new AuthenticationException(AuthenticationError.NETWORK))
                .doNothing()
                .when(gateway)
                .logout("refresh-initial");
        assertThrows(AuthenticationException.class, sessions::logout);
        assertTrue(sessions.isAuthenticated());
        assertEquals("access-initial", sessions.accessToken());
        sessions.logout();
        assertFalse(sessions.isAuthenticated());
    }

    @Test
    void concurrentAccessRequestsRotateRefreshExactlyOnce() throws Exception {
        signIn(10);
        CountDownLatch entered = new CountDownLatch(1);
        CountDownLatch release = new CountDownLatch(1);
        when(gateway.refresh("refresh-initial"))
                .thenAnswer(
                        invocation -> {
                            entered.countDown();
                            assertTrue(release.await(5, TimeUnit.SECONDS));
                            return tokens("renewed", 900);
                        });
        var executor = Executors.newFixedThreadPool(2);
        try {
            var first = executor.submit(sessions::accessToken);
            assertTrue(entered.await(5, TimeUnit.SECONDS));
            var second = executor.submit(sessions::accessToken);
            release.countDown();
            assertEquals("access-renewed", first.get(5, TimeUnit.SECONDS));
            assertEquals("access-renewed", second.get(5, TimeUnit.SECONDS));
            verify(gateway, times(1)).refresh("refresh-initial");
            sessions.logout();
            verify(gateway).logout("refresh-renewed");
        } finally {
            release.countDown();
            executor.shutdownNow();
        }
    }

    @Test
    void anUncertainRefreshIsNeverReplayed() throws Exception {
        signIn(10);
        when(gateway.refresh("refresh-initial"))
                .thenThrow(new AuthenticationException(AuthenticationError.TIMEOUT));
        assertThrows(AuthenticationException.class, sessions::accessToken);
        assertFalse(sessions.isAuthenticated());
        assertThrows(AuthenticationException.class, sessions::accessToken);
        verify(gateway, times(1)).refresh("refresh-initial");
    }

    @Test
    void safeUnavailableRefreshCanBeRetriedWithoutLosingTheSession() throws Exception {
        signIn(10);
        when(gateway.refresh("refresh-initial"))
                .thenThrow(new AuthenticationException(AuthenticationError.UNAVAILABLE))
                .thenReturn(tokens("renewed", 900));
        assertThrows(AuthenticationException.class, sessions::accessToken);
        assertTrue(sessions.isAuthenticated());
        assertEquals("access-renewed", sessions.accessToken());
    }

    @Test
    void refreshCannotChangeIdentityOrExtendTheAbsoluteSessionExpiry() throws Exception {
        signIn(10);
        SessionTokens replacement =
                new SessionTokens(
                        tokens("other", 900).access(),
                        "refresh-other",
                        IDENTITY,
                        NOW.plus(Duration.ofDays(21)));
        when(gateway.refresh("refresh-initial")).thenReturn(replacement);
        assertThrows(AuthenticationException.class, sessions::accessToken);
        assertFalse(sessions.isAuthenticated());
    }

    @Test
    void absoluteSessionExpiryDisablesAccessWithoutRefreshing() throws Exception {
        signIn(900);
        time.set(NOW.plus(Duration.ofDays(21)));
        assertFalse(sessions.isAuthenticated());
        assertThrows(AuthenticationException.class, sessions::accessToken);
        verify(gateway, never()).refresh(any());
    }

    @Test
    void closingTheApplicationRejectsLateLoginResultsWithoutRemoteLogout() throws Exception {
        when(gateway.login(any()))
                .thenAnswer(
                        invocation -> {
                            sessions.close();
                            return tokens("initial", 900);
                        });
        assertThrows(
                AuthenticationException.class,
                () -> sessions.login(new LoginCredentials("user@example.invalid", "password")));
        assertFalse(sessions.isAuthenticated());
        verify(gateway, never()).logout(any());
    }

    @Test
    void credentialObjectsNeverExposeTheirContentsThroughToString() {
        assertFalse(tokens("private", 900).toString().contains("private"));
        assertFalse(tokens("private", 900).access().toString().contains("private"));
        String credentials = new LoginCredentials("user@example.invalid", "private").toString();
        assertFalse(credentials.contains("user@example.invalid"));
        assertFalse(credentials.contains("private"));
    }

    private void signIn(long accessLifetime) throws Exception {
        when(gateway.login(any())).thenReturn(tokens("initial", accessLifetime));
        sessions.login(new LoginCredentials("user@example.invalid", "password"));
    }

    private static SessionTokens tokens(String label, long accessLifetime) {
        return new SessionTokens(
                new SessionTokens.AccessToken("access-" + label, NOW.plusSeconds(accessLifetime)),
                "refresh-" + label,
                IDENTITY,
                NOW.plus(Duration.ofDays(20)));
    }
}
