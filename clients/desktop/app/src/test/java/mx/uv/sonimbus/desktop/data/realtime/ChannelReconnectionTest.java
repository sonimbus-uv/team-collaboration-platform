package mx.uv.sonimbus.desktop.data.realtime;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import java.net.http.HttpClient;
import java.time.Clock;
import java.time.Duration;
import java.time.Instant;
import java.time.ZoneOffset;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.Semaphore;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import mx.uv.sonimbus.desktop.data.core.Message;
import mx.uv.sonimbus.desktop.data.session.Session;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationGateway;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.domain.session.SessionTokens;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class ChannelReconnectionTest {

    private static final class AdjustableClock extends Clock {
        private volatile Instant now = Instant.parse("2026-11-20T10:00:00Z");

        void advance(Duration amount) {
            now = now.plus(amount);
        }

        @Override
        public java.time.ZoneId getZone() {
            return ZoneOffset.UTC;
        }

        @Override
        public Clock withZone(java.time.ZoneId zone) {
            return this;
        }

        @Override
        public Instant instant() {
            return now;
        }
    }

    private static final class Events implements ChannelConnection.Listener {
        final Semaphore joined = new Semaphore(0);
        final Semaphore disconnected = new Semaphore(0);
        final AtomicInteger joins = new AtomicInteger();

        @Override
        public void onJoined() {
            joins.incrementAndGet();
            joined.release();
        }

        @Override
        public void onMessage(Message message) {
        }

        @Override
        public void onDisconnected() {
            disconnected.release();
        }

        @Override
        public void onAccessDenied() {
        }
    }

    private static final Instant SESSION_END = Instant.parse("2026-11-21T10:00:00Z");

    private final ObjectMapper json = new ObjectMapper()
            .setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
            .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);
    private final AdjustableClock clock = new AdjustableClock();
    private final Events events = new Events();
    private FakeCoreSocket core;
    private ChannelConnection connection;

    @BeforeEach
    void start() throws Exception {
        core = new FakeCoreSocket();
        AuthenticationGateway gateway = mock(AuthenticationGateway.class);
        SessionService sessions = new SessionService(gateway, clock);
        UUID account = UUID.randomUUID();
        UUID sid = UUID.randomUUID();
        AtomicInteger issued = new AtomicInteger();
        when(gateway.refresh("refresh-0")).thenAnswer(call -> tokens(account, sid, "access-" + issued.incrementAndGet()));
        sessions.signInFederated(tokens(account, sid, "access-0"));
        connection = new ChannelConnection(HttpClient.newHttpClient(), core.url(),
                new Session(sessions, () -> { }), "c1", events, json);
    }

    private SessionTokens tokens(UUID account, UUID sid, String access) {
        return new SessionTokens(new SessionTokens.AccessToken(access, clock.instant().plusSeconds(900)),
                "refresh-0", new SessionTokens.Identity(sid, account), SESSION_END);
    }

    @AfterEach
    void stop() throws Exception {
        connection.close();
        core.close();
    }

    private boolean await(Semaphore semaphore) throws InterruptedException {
        return semaphore.tryAcquire(15, TimeUnit.SECONDS);
    }

    @Test
    void joinsTheChannelCarryingTheTokenInTheSubprotocol() throws Exception {
        connection.connect();

        assertTrue(await(events.joined));
        assertTrue(core.subprotocols().get(0).contains(ChannelConnection.AUTH_TOKEN_PREFIX
                + ChannelConnection.encodeToken("access-0")));
    }

    @Test
    void reconnectsAfterTheServerDropsTheSocket() throws Exception {
        connection.connect();
        assertTrue(await(events.joined));

        core.dropConnection();

        assertTrue(await(events.disconnected));
        assertTrue(await(events.joined));
        assertEquals(2, events.joins.get());
    }

    @Test
    void tokenExpiredClosesTheChannelAndReconnectsWithARenewedToken() throws Exception {
        connection.connect();
        assertTrue(await(events.joined));
        clock.advance(Duration.ofMinutes(20));

        core.push("token.expired", "{}");

        assertTrue(await(events.disconnected));
        assertTrue(await(events.joined));
        assertEquals(2, core.subprotocols().size());
        assertTrue(core.subprotocols().get(1).contains(ChannelConnection.encodeToken("access-1")));
        assertFalse(core.subprotocols().get(1).contains(ChannelConnection.encodeToken("access-0")));
    }

    @Test
    void sendWhileDisconnectedFailsWithoutReachingTheServer() throws Exception {
        CompletableFuture<Message> result = connection.send("k1", "hola");

        assertTrue(result.isCompletedExceptionally());
    }

    @Test
    void messageLeftPendingByADropIsResentWithTheSameClientMessageId() throws Exception {
        connection.connect();
        assertTrue(await(events.joined));
        CompletableFuture<Message> first = connection.send("k1", "hola");
        JsonNode firstFrame = core.nextFrame("message.create");

        core.dropConnection();
        assertTrue(await(events.disconnected));
        assertTrue(await(events.joined));
        AtomicReference<JsonNode> secondFrame = new AtomicReference<>();
        connection.send("k1", "hola");
        secondFrame.set(core.nextFrame("message.create"));

        assertTrue(first.isCompletedExceptionally());
        assertEquals("k1", firstFrame.get(4).get("client_message_id").asText());
        assertEquals("k1", secondFrame.get().get(4).get("client_message_id").asText());
    }
}
