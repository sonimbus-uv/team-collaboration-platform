package mx.uv.sonimbus.desktop.data.core;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.time.Clock;
import java.time.Instant;
import java.time.ZoneOffset;
import java.util.ArrayList;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.atomic.AtomicInteger;
import mx.uv.sonimbus.desktop.data.api.ApiConfig;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.session.Session;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationGateway;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.domain.session.SessionTokens;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class CoreApiTest {

    private static final String GROUP = """
            {"id":"g1","name":"Equipo","description":null,"image_id":null,"created_by":"u1",\
            "role":"admin","archived_at":null,"created_at":"2026-10-29T12:00:00Z"}""";

    private HttpServer server;
    private CoreApi api;
    private final List<String> authorizations = new ArrayList<>();
    private final AtomicInteger refreshes = new AtomicInteger();
    private String requestMethod;
    private String requestPath;
    private String requestQuery;
    private String requestBody;

    @BeforeEach
    void start() throws Exception {
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.start();
        Instant now = Instant.parse("2026-10-29T12:00:00Z");
        SessionTokens.Identity identity = new SessionTokens.Identity(UUID.randomUUID(), UUID.randomUUID());
        AuthenticationGateway gateway = org.mockito.Mockito.mock(AuthenticationGateway.class);
        org.mockito.Mockito.when(gateway.refresh("refresh-old")).thenAnswer(call -> {
            refreshes.incrementAndGet();
            return new SessionTokens(new SessionTokens.AccessToken("access-new", now.plusSeconds(900)),
                    "refresh-new", identity, now.plusSeconds(3600));
        });
        SessionService sessions = new SessionService(gateway, Clock.fixed(now, ZoneOffset.UTC));
        sessions.signInFederated(new SessionTokens(
                new SessionTokens.AccessToken("access-old", now.plusSeconds(900)),
                "refresh-old", identity, now.plusSeconds(3600)));
        Session session = new Session(sessions, () -> { });
        URI base = URI.create("http://127.0.0.1:" + server.getAddress().getPort());
        api = new CoreApi(ApiConfig.withUrls(URI.create("http://auth.invalid"), base), session);
    }

    @AfterEach
    void stop() {
        server.stop(0);
    }

    @Test
    void listGroupsSendsBearerAndReadsData() throws ApiException {
        respond(200, "{\"data\":[" + GROUP + "]}");

        List<Group> groups = api.listGroups();

        assertEquals("GET", requestMethod);
        assertEquals("/api/v1/groups", requestPath);
        assertEquals(List.of("Bearer access-old"), authorizations);
        assertEquals("Equipo", groups.get(0).name());
        assertTrue(groups.get(0).isAdmin());
    }

    @Test
    void rejectedTokenIsRenewedAndRetriedOnce() throws ApiException {
        server.createContext("/", exchange -> {
            String auth = exchange.getRequestHeaders().getFirst("Authorization");
            send(exchange, auth.endsWith("access-old") ? 401 : 200,
                    auth.endsWith("access-old") ? problem("unauthorized", 401) : "{\"data\":[]}");
        });

        assertEquals(List.of(), api.listGroups());
        assertEquals(List.of("Bearer access-old", "Bearer access-new"), authorizations);
        assertEquals(1, refreshes.get());
    }

    @Test
    void createGroupOmitsMissingDescription() throws ApiException {
        respond(201, "{\"data\":" + GROUP + "}");

        api.createGroup("Equipo", null);

        assertEquals("POST", requestMethod);
        assertEquals("{\"name\":\"Equipo\"}", requestBody);
    }

    @Test
    void joinErrorsAreDistinguishedByProblemType() {
        respond(422, problem("invitation-invalid", 422));
        assertEquals(ApiErrorKind.INVALID_INVITATION,
                assertThrows(ApiException.class, () -> api.joinGroup("abc")).kind());

        respond(403, problem("banned", 403));
        assertEquals(ApiErrorKind.BANNED, assertThrows(ApiException.class, () -> api.joinGroup("abc")).kind());

        respond(422, problem("validation-error", 422));
        assertEquals(ApiErrorKind.VALIDATION, assertThrows(ApiException.class, () -> api.joinGroup("abc")).kind());
    }

    @Test
    void joinSendsCodeInBodyNotInUrl() throws ApiException {
        respond(201, "{\"data\":" + GROUP + "}");

        api.joinGroup("code-secret");

        assertEquals("/api/v1/invitations/join", requestPath);
        assertEquals("{\"code\":\"code-secret\"}", requestBody);
    }

    @Test
    void missingProfileIsEmpty() throws ApiException {
        respond(404, problem("not-found", 404));

        assertTrue(api.myProfile().isEmpty());
    }

    @Test
    void messagesPageReadsCursorAndAuthor() throws ApiException {
        respond(200, """
                {"data":[{"id":"m1","channel_id":"c1","author_id":"u1",\
                "author":{"id":"u1","display_name":"Ana"},"client_message_id":"k1",\
                "content":"Hola","reply_to":null,"file_id":null,\
                "created_at":"2026-10-29T15:04:05.123456Z","edited_at":null,"deleted":false}],\
                "next_cursor":null}""");

        MessagePage page = api.listMessages("c1", "m9", 50);

        assertEquals("/api/v1/channels/c1/messages", requestPath);
        assertEquals("limit=50&before=m9", requestQuery);
        assertEquals("Ana", page.data().get(0).author().displayName());
        assertEquals("2026-10-29T15:04:05.123456Z", page.data().get(0).createdInstant().toString());
        assertNull(page.nextCursor());
    }

    @Test
    void errorsDoNotExposeServerDetail() {
        respond(500, "{\"type\":\"/problems/internal\",\"status\":500,\"detail\":\"SELECT * FROM core.groups\"}");

        ApiException e = assertThrows(ApiException.class, api::listGroups);

        assertEquals(ApiErrorKind.SERVER, e.kind());
        assertFalse(e.getMessage().contains("SELECT"), e.getMessage());
    }

    private static String problem(String type, int status) {
        return "{\"type\":\"/problems/" + type + "\",\"title\":\"x\",\"status\":" + status + "}";
    }

    private void respond(int status, String body) {
        try {
            server.removeContext("/");
        } catch (IllegalArgumentException e) {
            // Primera respuesta de la prueba.
        }
        server.createContext("/", exchange -> send(exchange, status, body));
    }

    private void send(HttpExchange exchange, int status, String body) throws IOException {
        requestMethod = exchange.getRequestMethod();
        requestPath = exchange.getRequestURI().getPath();
        requestQuery = exchange.getRequestURI().getRawQuery();
        requestBody = new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8);
        authorizations.add(exchange.getRequestHeaders().getFirst("Authorization"));
        byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().add("Content-Type", "application/json");
        exchange.sendResponseHeaders(status, bytes.length);
        exchange.getResponseBody().write(bytes);
        exchange.close();
    }
}
