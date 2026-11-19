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
import java.util.concurrent.atomic.AtomicBoolean;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.Tokens;
import org.junit.jupiter.api.Test;

class SessionTest {

    private final MutableClock clock = new MutableClock();
    private final List<String> refreshedWith = new ArrayList<>();
    private int issued;

    private Tokens tokens(String suffix) {
        return new Tokens("access-" + suffix, "refresh-" + suffix, "Bearer", 900,
                "2026-11-18T00:00:00Z", "sid", "user-1");
    }

    private Session session() {
        Session session = new Session(refresh -> {
            refreshedWith.add(refresh);
            return tokens(Integer.toString(++issued));
        }, clock);
        session.start(tokens("0"));
        return session;
    }

    @Test
    void freshTokenIsUsedAsIs() throws ApiException {
        Session session = session();

        assertEquals("access-0", session.accessToken());
        assertEquals("user-1", session.userId());
        assertTrue(refreshedWith.isEmpty());
    }

    @Test
    void tokenCloseToExpiryIsRenewedFirst() throws ApiException {
        Session session = session();
        clock.advance(Duration.ofSeconds(900).minus(Session.REFRESH_MARGIN).plusSeconds(1));

        assertEquals("access-1", session.accessToken());
        assertEquals(List.of("refresh-0"), refreshedWith);
    }

    @Test
    void rejectionRenewsOnlyOnceForTheSameToken() throws ApiException {
        Session session = session();

        assertEquals("access-1", session.renewAfterRejection("access-0"));
        assertEquals("access-1", session.renewAfterRejection("access-0"), "otro hilo ya lo renovó");
        assertEquals(1, refreshedWith.size());
    }

    @Test
    void rejectedRefreshEndsTheSession() {
        AtomicBoolean expired = new AtomicBoolean();
        Session session = new Session(refresh -> {
            throw new ApiException(ApiErrorKind.UNAUTHORIZED, "refresh", 401, null);
        }, clock);
        session.setOnExpired(() -> expired.set(true));
        session.start(tokens("0"));

        assertThrows(ApiException.class, () -> session.renewAfterRejection("access-0"));
        assertTrue(expired.get());
        assertFalse(session.isActive());
    }

    @Test
    void networkErrorKeepsTheSession() {
        Session session = new Session(refresh -> {
            throw new ApiException(ApiErrorKind.NETWORK, "refresh", null);
        }, clock);
        session.start(tokens("0"));

        assertThrows(ApiException.class, () -> session.renewAfterRejection("access-0"));
        assertTrue(session.isActive());
    }

    @Test
    void withoutSessionItIsUnauthorized() {
        Session session = new Session(refresh -> tokens("x"), clock);

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
