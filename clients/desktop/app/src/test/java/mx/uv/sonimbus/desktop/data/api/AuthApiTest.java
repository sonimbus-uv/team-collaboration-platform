package mx.uv.sonimbus.desktop.data.api;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.Map;
import java.util.Optional;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;

class AuthApiTest {

    private static final String ACCOUNT = """
            {"user_id":"7b0c","email":"ana@uv.mx","email_verified":%s,"status":"active",\
            "verification_email_sent":%s}""";

    private HttpServer server;
    private AuthApi api;
    private String requestMethod;
    private String requestPath;
    private String requestContentType;
    private String requestBody;

    @BeforeEach
    void startServer() throws IOException {
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.start();
        api = new AuthApi(URI.create("http://127.0.0.1:" + server.getAddress().getPort() + "/"));
    }

    @AfterEach
    void stopServer() {
        server.stop(0);
    }

    @Test
    void registerSendsCredentialsAndReadsAccount() throws ApiException, IOException {
        respond(201, ACCOUNT.formatted(false, true));

        Account account = api.register("ana@uv.mx", "ValidPass123!");

        assertEquals("POST", requestMethod);
        assertEquals("/auth/register", requestPath);
        assertEquals("application/json", requestContentType);
        assertEquals(Map.of("email", "ana@uv.mx", "password", "ValidPass123!"), sentJson());
        assertEquals("7b0c", account.userId());
        assertEquals("ana@uv.mx", account.email());
        assertFalse(account.emailVerified());
        assertEquals("active", account.status());
        assertTrue(account.verificationEmailSent());
    }

    @Test
    void registerReportsEmailNotSent() throws ApiException {
        respond(201, ACCOUNT.formatted(false, false));

        assertFalse(api.register("ana@uv.mx", "ValidPass123!").verificationEmailSent());
    }

    @Test
    void duplicatedEmailIsConflict() {
        respond(409, problem(409, "Conflict", "Email already exists"));

        ApiException e = assertThrows(ApiException.class, () -> api.register("ana@uv.mx", "ValidPass123!"));

        assertEquals(ApiError.EMAIL_ALREADY_EXISTS, e.error());
        assertEquals(409, e.status());
    }

    @Test
    void validationErrorsAreDistinguished() {
        assertEquals(ApiError.INVALID_EMAIL, registerErrorFor("Invalid email format"));
        assertEquals(ApiError.INVALID_PASSWORD, registerErrorFor("Password is too short"));
        assertEquals(ApiError.INVALID_PASSWORD, registerErrorFor("Password is too long"));
        assertEquals(ApiError.INVALID_PASSWORD, registerErrorFor("Invalid password format"));
        assertEquals(ApiError.INVALID_REQUEST, registerErrorFor("Something new"));
    }

    @Test
    void tooManyRequestsCarriesRetryAfter() {
        server.createContext("/", exchange -> {
            exchange.getResponseHeaders().add("Retry-After", "60");
            send(exchange, 429, problem(429, "Too many requests", "Please retry later."));
        });

        ApiException e = assertThrows(ApiException.class, () -> api.register("ana@uv.mx", "ValidPass123!"));

        assertEquals(ApiError.TOO_MANY_REQUESTS, e.error());
        assertEquals(Optional.of(Duration.ofSeconds(60)), e.retryAfter());
    }

    @Test
    void verifyEmailSendsTokenAndReadsAccount() throws ApiException {
        respond(200, ACCOUNT.formatted(true, false));

        Account account = api.verifyEmail("abc123");

        assertEquals("/auth/verify-email", requestPath);
        assertEquals("{\"token\":\"abc123\"}", requestBody);
        assertTrue(account.emailVerified());
    }

    @Test
    void invalidTokenIsReported() {
        respond(400, problem(400, "Invalid request", "Invalid or expired verification token"));

        ApiException e = assertThrows(ApiException.class, () -> api.verifyEmail("abc123"));

        assertEquals(ApiError.INVALID_TOKEN, e.error());
    }

    @Test
    void resendVerificationAcceptsEmptyResponse() throws ApiException {
        respond(202, "");

        api.resendVerification("ana@uv.mx");

        assertEquals("/auth/resend-verification", requestPath);
        assertEquals("{\"email\":\"ana@uv.mx\"}", requestBody);
    }

    @Test
    void serverErrorsAreNotMistakenForUserErrors() {
        respond(503, problem(503, "Database error", "The service cannot complete the database operation."));
        assertEquals(ApiError.SERVICE_UNAVAILABLE,
                assertThrows(ApiException.class, () -> api.resendVerification("ana@uv.mx")).error());
    }

    @Test
    void errorWithoutProblemBodyIsUnexpected() {
        respond(500, "<html>Internal Server Error</html>");

        ApiException e = assertThrows(ApiException.class, () -> api.verifyEmail("abc123"));

        assertEquals(ApiError.UNEXPECTED, e.error());
        assertEquals(500, e.status());
    }

    @Test
    void successWithUnreadableBodyIsUnexpected() {
        respond(201, "not json");

        ApiException e = assertThrows(ApiException.class, () -> api.register("ana@uv.mx", "ValidPass123!"));

        assertEquals(ApiError.UNEXPECTED, e.error());
    }

    @Test
    void unreachableServiceIsNetworkError() {
        server.stop(0);

        ApiException e = assertThrows(ApiException.class, () -> api.register("ana@uv.mx", "ValidPass123!"));

        assertEquals(ApiError.NETWORK, e.error());
        assertEquals(0, e.status());
    }

    private ApiError registerErrorFor(String detail) {
        respond(400, problem(400, "Invalid request", detail));
        return assertThrows(ApiException.class, () -> api.register("ana@uv.mx", "x")).error();
    }

    private void respond(int status, String body) {
        try {
            server.removeContext("/");
        } catch (IllegalArgumentException e) {
            // Primera respuesta de la prueba: aún no hay contexto.
        }
        server.createContext("/", exchange -> send(exchange, status, body));
    }

    private void send(HttpExchange exchange, int status, String body) throws IOException {
        requestMethod = exchange.getRequestMethod();
        requestPath = exchange.getRequestURI().getPath();
        requestContentType = exchange.getRequestHeaders().getFirst("Content-Type");
        requestBody = new String(exchange.getRequestBody().readAllBytes(), StandardCharsets.UTF_8);
        byte[] bytes = body.getBytes(StandardCharsets.UTF_8);
        exchange.getResponseHeaders().add("Content-Type", "application/json");
        exchange.sendResponseHeaders(status, bytes.length == 0 ? -1 : bytes.length);
        if (bytes.length > 0) {
            exchange.getResponseBody().write(bytes);
        }
        exchange.close();
    }

    private static String problem(int status, String title, String detail) {
        return "{\"type\":\"about:blank\",\"title\":\"%s\",\"status\":%d,\"detail\":\"%s\"}"
                .formatted(title, status, detail);
    }

    private Map<String, String> sentJson() throws IOException {
        return new ObjectMapper().readValue(requestBody, new TypeReference<>() { });
    }
}
