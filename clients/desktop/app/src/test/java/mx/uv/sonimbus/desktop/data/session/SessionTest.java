package mx.uv.sonimbus.desktop.data.session;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.time.Clock;
import java.time.Duration;
import java.time.Instant;
import java.time.ZoneId;
import java.time.ZoneOffset;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicBoolean;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationError;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationGateway;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.domain.session.SessionTokens;
import org.junit.jupiter.api.Test;

class SessionTest {

    private final MutableClock clock = new MutableClock();
    private final List<String> refreshedWith = new ArrayList<>();
    private int issued;

    private final SessionTokens.Identity identity =
            new SessionTokens.Identity(UUID.randomUUID(), UUID.randomUUID());
    private final SessionTokens initial = tokens("0");

    private SessionTokens tokens(String suffix) {
        return new SessionTokens(new SessionTokens.AccessToken("access-" + suffix, clock.instant().plusSeconds(900)),
                "refresh-" + suffix, identity, Instant.parse("2026-11-18T00:00:00Z"));
    }

    private Session session() throws AuthenticationException {
        AuthenticationGateway gateway = mock(AuthenticationGateway.class);
        when(gateway.refresh("refresh-0")).thenAnswer(call -> {
            refreshedWith.add(call.getArgument(0));
            return tokens(Integer.toString(++issued));
        });
        SessionService sessions = new SessionService(gateway, clock);
        sessions.signInFederated(initial);
        return new Session(sessions, () -> { });
    }

    @Test
    void freshTokenIsUsedAsIs() throws AuthenticationException, ApiException {
        Session session = session();

        assertEquals("access-0", session.accessToken());
        assertTrue(refreshedWith.isEmpty());
    }

    @Test
    void tokenCloseToExpiryIsRenewedFirst() throws AuthenticationException, ApiException {
        Session session = session();
        clock.advance(Duration.ofSeconds(871));

        assertEquals("access-1", session.accessToken());
        assertEquals(List.of("refresh-0"), refreshedWith);
    }

    @Test
    void rejectionRenewsOnlyOnceForTheSameToken() throws AuthenticationException, ApiException {
        Session session = session();

        assertEquals("access-1", session.renewAfterRejection("access-0"));
        assertEquals("access-1", session.renewAfterRejection("access-0"), "otro hilo ya lo renovó");
        assertEquals(1, refreshedWith.size());
    }

    @Test
    void rejectedRefreshEndsTheSession() throws AuthenticationException {
        AtomicBoolean expired = new AtomicBoolean();
        AuthenticationGateway gateway = mock(AuthenticationGateway.class);
        when(gateway.refresh("refresh-0")).thenThrow(new AuthenticationException(AuthenticationError.NO_SESSION));
        SessionService sessions = new SessionService(gateway, clock);
        sessions.signInFederated(initial);
        Session session = new Session(sessions, () -> expired.set(true));

        assertThrows(ApiException.class, () -> session.renewAfterRejection("access-0"));
        assertTrue(expired.get());
        assertFalse(sessions.isAuthenticated());
    }

    @Test
    void unavailableRefreshKeepsTheSession() throws AuthenticationException {
        AuthenticationGateway gateway = mock(AuthenticationGateway.class);
        when(gateway.refresh("refresh-0")).thenThrow(new AuthenticationException(AuthenticationError.UNAVAILABLE));
        SessionService sessions = new SessionService(gateway, clock);
        sessions.signInFederated(initial);
        Session session = new Session(sessions, () -> { });

        assertThrows(ApiException.class, () -> session.renewAfterRejection("access-0"));
        assertTrue(sessions.isAuthenticated());
    }

    @Test
    void withoutSessionItIsUnauthorized() {
        Session session = new Session(new SessionService(mock(AuthenticationGateway.class), clock), () -> { });

        assertEquals(ApiErrorKind.UNAUTHORIZED,
                assertThrows(ApiException.class, session::accessToken).kind());
    }

    private static final class MutableClock extends Clock {
        private Instant now = Instant.parse("2026-10-29T12:00:00Z");

        void advance(Duration duration) {
            now = now.plus(duration);
        }

        @Override
        public ZoneOffset getZone() {
            return ZoneOffset.UTC;
        }

        @Override
        public Clock withZone(ZoneId zone) {
            return this;
        }

        @Override
        public Instant instant() {
            return now;
        }
    }
}
