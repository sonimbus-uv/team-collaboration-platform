package mx.uv.sonimbus.desktop.domain.google;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.ArgumentMatchers.eq;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.never;
import static org.mockito.Mockito.times;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.when;

import mx.uv.sonimbus.desktop.domain.session.AuthenticationGateway;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.domain.session.SessionTokens;

import org.junit.jupiter.api.Test;
import org.mockito.ArgumentCaptor;

import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.time.Instant;
import java.util.Base64;
import java.util.UUID;

class GoogleServiceTest {
    private static final String SECRET = "A".repeat(43);
    private static final UUID ID = UUID.randomUUID();
    private final GoogleGateway gateway = mock(GoogleGateway.class);
    private final GoogleBrowser browser = mock(GoogleBrowser.class);
    private final SessionService sessions = new SessionService(mock(AuthenticationGateway.class));
    private final GoogleService google = new GoogleService(gateway, browser, sessions);

    private GoogleAttempt attempt() {
        return new GoogleAttempt(
                ID,
                SECRET,
                URI.create(
                        "https://auth.example.invalid/api/v1/auth/google/authorize?ticket="
                                + SECRET),
                Instant.now().plusSeconds(600));
    }

    @Test
    void completesPkceAndInstallsNormalSessionWithoutInventingEmail() throws Exception {
        when(gateway.createAttempt(any())).thenReturn(attempt());
        when(browser.authorize(any(), any())).thenReturn(SECRET);
        when(gateway.exchange(any())).thenReturn(tokens());
        google.signIn();
        ArgumentCaptor<String> challenge = ArgumentCaptor.forClass(String.class);
        ArgumentCaptor<GoogleExchange> proof = ArgumentCaptor.forClass(GoogleExchange.class);
        verify(gateway).createAttempt(challenge.capture());
        verify(gateway).exchange(proof.capture());
        assertEquals(ID, proof.getValue().attemptId());
        assertEquals(SECRET, proof.getValue().attemptSecret());
        assertTrue(proof.getValue().codeVerifier().matches("[A-Za-z0-9_-]{43}"));
        String expected =
                Base64.getUrlEncoder()
                        .withoutPadding()
                        .encodeToString(
                                MessageDigest.getInstance("SHA-256")
                                        .digest(
                                                proof.getValue()
                                                        .codeVerifier()
                                                        .getBytes(StandardCharsets.US_ASCII)));
        assertEquals(expected, challenge.getValue());
        assertTrue(sessions.isAuthenticated());
        assertTrue(sessions.email().isEmpty());
        assertEquals("access", sessions.accessToken());
    }

    @Test
    void cancelledCallbackNeverExchangesOrInstallsSession() throws Exception {
        when(gateway.createAttempt(any())).thenReturn(attempt());
        when(browser.authorize(any(), any())).thenThrow(new GoogleException(GoogleError.CANCELLED));
        assertEquals(
                GoogleError.CANCELLED, assertThrows(GoogleException.class, google::signIn).kind());
        verify(gateway, never()).exchange(any());
        assertFalse(sessions.isAuthenticated());
    }

    @Test
    void lateCallbackAfterCancelNeverExchanges() throws Exception {
        when(gateway.createAttempt(any())).thenReturn(attempt());
        when(browser.authorize(any(), any()))
                .thenAnswer(
                        call -> {
                            google.cancel();
                            return SECRET;
                        });
        assertEquals(
                GoogleError.CANCELLED, assertThrows(GoogleException.class, google::signIn).kind());
        verify(gateway, never()).exchange(any());
    }

    @Test
    void uncertainExchangeIsNeverReplayed() throws Exception {
        when(gateway.createAttempt(any())).thenReturn(attempt());
        when(browser.authorize(any(), any())).thenReturn(SECRET);
        when(gateway.exchange(any())).thenThrow(new GoogleException(GoogleError.UNCERTAIN));
        assertThrows(GoogleException.class, google::signIn);
        verify(gateway, times(1)).exchange(any());
        assertFalse(sessions.isAuthenticated());
    }

    @Test
    void expiredAttemptsNeverOpenBrowser() throws Exception {
        when(gateway.createAttempt(any()))
                .thenReturn(
                        new GoogleAttempt(
                                ID,
                                SECRET,
                                URI.create("https://example.invalid"),
                                Instant.now().minusSeconds(1)));
        assertEquals(
                GoogleError.EXPIRED, assertThrows(GoogleException.class, google::signIn).kind());
        verify(browser, never()).authorize(any(), any());
    }

    @Test
    void closePreventsLateSessionInstallation() throws Exception {
        when(gateway.createAttempt(any())).thenReturn(attempt());
        when(browser.authorize(any(), any())).thenReturn(SECRET);
        when(gateway.exchange(any()))
                .thenAnswer(
                        call -> {
                            sessions.close();
                            return tokens();
                        });
        assertThrows(GoogleException.class, google::signIn);
        assertFalse(sessions.isAuthenticated());
    }

    @Test
    void descriptionsRedactAllProofs() {
        assertFalse(attempt().toString().contains(SECRET));
        assertFalse(new GoogleExchange(ID, SECRET, SECRET, SECRET).toString().contains(SECRET));
    }

    private void signedInSession() throws Exception {
        sessions.signInFederated(tokens());
    }

    @Test
    void linkPreservesTheCurrentSessionAndUsesOnlyLinkExchange() throws Exception {
        signedInSession();
        String access = sessions.accessToken();
        when(gateway.createLinkAttempt(eq(access), any())).thenReturn(attempt());
        when(browser.authorize(any(), any())).thenReturn(SECRET);
        google.link();
        verify(gateway).createLinkAttempt(eq(access), any());
        verify(gateway).exchangeLink(any());
        verify(gateway, never()).exchange(any());
        verify(gateway, never()).reauthenticate(any(), any());
        assertEquals(access, sessions.accessToken());
        assertTrue(sessions.isAuthenticated());
    }

    @Test
    void expiredEvidenceRequestsConfirmationBeforeOpeningGoogle() throws Exception {
        signedInSession();
        when(gateway.createLinkAttempt(any(), any()))
                .thenThrow(new GoogleException(GoogleError.REAUTHENTICATION_REQUIRED));
        assertEquals(
                GoogleError.REAUTHENTICATION_REQUIRED,
                assertThrows(GoogleException.class, google::link).kind());
        verify(browser, never()).authorize(any(), any());
        assertTrue(sessions.isAuthenticated());
    }

    @Test
    void confirmedPasswordIsReauthenticatedBeforeCreatingTheLink() throws Exception {
        signedInSession();
        when(gateway.createLinkAttempt(any(), any())).thenReturn(attempt());
        when(browser.authorize(any(), any())).thenReturn(SECRET);
        google.link(" current password ");
        var order = org.mockito.Mockito.inOrder(gateway);
        order.verify(gateway).reauthenticate("access", " current password ");
        order.verify(gateway).createLinkAttempt(eq("access"), any());
        order.verify(gateway).exchangeLink(any());
        assertEquals("access", sessions.accessToken());
    }

    @Test
    void wrongConfirmationDoesNotOpenGoogleOrClearSession() throws Exception {
        signedInSession();
        doThrow(new GoogleException(GoogleError.INVALID_PASSWORD))
                .when(gateway)
                .reauthenticate(any(), any());
        assertEquals(
                GoogleError.INVALID_PASSWORD,
                assertThrows(GoogleException.class, () -> google.link("wrong")).kind());
        verify(gateway, never()).createLinkAttempt(any(), any());
        verify(browser, never()).authorize(any(), any());
        assertTrue(sessions.isAuthenticated());
    }

    @Test
    void cancelledLinkNeverExchangesOrReplacesTheCurrentSession() throws Exception {
        signedInSession();
        when(gateway.createLinkAttempt(any(), any())).thenReturn(attempt());
        when(browser.authorize(any(), any())).thenThrow(new GoogleException(GoogleError.CANCELLED));
        assertThrows(GoogleException.class, google::link);
        verify(gateway, never()).exchangeLink(any());
        assertEquals("access", sessions.accessToken());
    }

    @Test
    void uncertainLinkExchangeIsNotRetriedAndKeepsTheExistingSession() throws Exception {
        signedInSession();
        when(gateway.createLinkAttempt(any(), any())).thenReturn(attempt());
        when(browser.authorize(any(), any())).thenReturn(SECRET);
        doThrow(new GoogleException(GoogleError.UNCERTAIN)).when(gateway).exchangeLink(any());
        assertThrows(GoogleException.class, google::link);
        verify(gateway, times(1)).exchangeLink(any());
        assertEquals("access", sessions.accessToken());
    }

    private SessionTokens tokens() {
        return new SessionTokens(
                new SessionTokens.AccessToken("access", Instant.now().plusSeconds(900)),
                "refresh",
                new SessionTokens.Identity(UUID.randomUUID(), UUID.randomUUID()),
                Instant.now().plusSeconds(3600));
    }
}
