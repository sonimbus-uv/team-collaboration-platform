package mx.uv.sonimbus.desktop.data.api;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import com.fasterxml.jackson.core.type.TypeReference;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;
import java.io.IOException;
import java.io.PrintWriter;
import java.io.StringWriter;
import java.net.InetSocketAddress;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.time.ZonedDateTime;
import java.time.format.DateTimeFormatter;
import java.util.Map;
import java.util.Optional;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

class AuthApiTest {

    private static final String PASSWORD = "ValidPass123!";
    private static final String ACCOUNT = """
            {"user_id":"7b0c","email":"ana@uv.mx","email_verified":%s,"status":"active",\
            "verification_email_sent":%s}""";

    private HttpServer server;
    private AuthApi api;
    private final CountDownLatch release = new CountDownLatch(1);
    private String requestMethod;
    private String requestPath;
    private String requestContentType;
    private String requestBody;

    @BeforeEach
    void startServer() throws IOException {
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.start();
        api = new AuthApi(ApiConfig.withAuthUrl(baseUrl()));
    }

    @AfterEach
    void stopServer() {
        release.countDown();
        server.stop(0);
    }

    // --- Respuestas correctas ---

    @Test
    void registerSendsCredentialsAndReadsAccount() throws ApiException, IOException {
        respond(201, ACCOUNT.formatted(false, true));

        Account account = api.register("ana@uv.mx", PASSWORD);

        assertEquals("POST", requestMethod);
        assertEquals("/auth/register", requestPath);
        assertEquals("application/json", requestContentType);
        assertEquals(Map.of("email", "ana@uv.mx", "password", PASSWORD), sentJson());
        assertEquals("7b0c", account.userId());
        assertEquals("ana@uv.mx", account.email());
        assertFalse(account.emailVerified());
        assertEquals("active", account.status());
        assertTrue(account.verificationEmailSent());
    }

    @Test
    void registerReportsEmailNotSent() throws ApiException {
        respond(201, ACCOUNT.formatted(false, false));

        assertFalse(api.register("ana@uv.mx", PASSWORD).verificationEmailSent());
    }

    @Test
    void unknownResponseFieldsAreIgnored() throws ApiException {
        respond(201, ACCOUNT.formatted(false, true).replace("}", ",\"display_name\":\"Ana\"}"));

        assertEquals("ana@uv.mx", api.register("ana@uv.mx", PASSWORD).email());
    }

    @Test
    void verifyEmailSendsTokenAndReadsAccount() throws ApiException, IOException {
        respond(200, ACCOUNT.formatted(true, false));

        Account account = api.verifyEmail("abc123");

        assertEquals("/auth/verify-email", requestPath);
        assertEquals(Map.of("token", "abc123"), sentJson());
        assertTrue(account.emailVerified());
    }

    @Test
    void resendVerificationAcceptsEmptyResponse() throws ApiException, IOException {
        respond(202, "");

        api.resendVerification("ana@uv.mx");

        assertEquals("/auth/resend-verification", requestPath);
        assertEquals(Map.of("email", "ana@uv.mx"), sentJson());
    }

    // --- Errores del servicio: se clasifican por código y operación, no por detail ---

    @Test
    void duplicatedEmailIsTaken() {
        respond(409, problem(409, "Conflict", "Email already exists"));

        ApiException e = registerFails();

        assertEquals(ApiErrorKind.EMAIL_TAKEN, e.kind());
        assertEquals(409, e.status());
    }

    @Test
    void conflictOutsideRegistrationIsUnexpected() {
        respond(409, problem(409, "Conflict", "Email already exists"));

        assertEquals(ApiErrorKind.UNEXPECTED, resendFails().kind());
    }

    @Test
    void anyBadRequestIsValidationWhateverTheDetail() {
        for (String detail : new String[] {"Invalid email format", "Password is too short", "Something new"}) {
            respond(400, problem(400, "Invalid request", detail));
            assertEquals(ApiErrorKind.VALIDATION, registerFails().kind(), detail);
        }
        respond(400, problem(400, "Invalid request", "Invalid email format"));
        assertEquals(ApiErrorKind.VALIDATION, resendFails().kind());
        respond(400, problem(400, "Invalid request", "Invalid or expired verification token"));
        assertEquals(ApiErrorKind.VALIDATION, assertThrows(ApiException.class, () -> api.verifyEmail("x")).kind());
    }

    @Test
    void detailIsNeverPartOfTheException() {
        respond(409, problem(409, "Conflict", "Email already exists"));

        ApiException e = registerFails();

        assertFalse(e.getMessage().contains("already exists"), e.getMessage());
        assertEquals("register: HTTP 409 (EMAIL_TAKEN)", e.getMessage());
    }

    @Test
    void tooManyRequestsCarriesRetryAfterSeconds() {
        server.createContext("/", exchange -> {
            exchange.getResponseHeaders().add("Retry-After", "60");
            send(exchange, 429, problem(429, "Too many requests", "Please retry later."));
        });

        ApiException e = registerFails();

        assertEquals(ApiErrorKind.RATE_LIMITED, e.kind());
        assertEquals(Optional.of(Duration.ofSeconds(60)), e.retryAfter());
    }

    @Test
    void tooManyRequestsWithoutRetryAfter() {
        respond(429, problem(429, "Too many requests", "Please retry later."));

        ApiException e = resendFails();

        assertEquals(ApiErrorKind.RATE_LIMITED, e.kind());
        assertEquals(Optional.empty(), e.retryAfter());
    }

    @Test
    void retryAfterAcceptsSecondsAndHttpDates() {
        assertEquals(Duration.ofSeconds(60), AuthApi.parseRetryAfter(" 60 "));
        assertEquals(Duration.ZERO, AuthApi.parseRetryAfter("-5"));
        assertNull(AuthApi.parseRetryAfter("pronto"));
        String inTwoMinutes = DateTimeFormatter.RFC_1123_DATE_TIME.format(ZonedDateTime.now().plusMinutes(2));
        long seconds = AuthApi.parseRetryAfter(inTwoMinutes).toSeconds();
        assertTrue(seconds > 100 && seconds <= 120, "segundos: " + seconds);
        String past = DateTimeFormatter.RFC_1123_DATE_TIME.format(ZonedDateTime.now().minusMinutes(2));
        assertEquals(Duration.ZERO, AuthApi.parseRetryAfter(past));
    }

    @Test
    void internalErrorIsServer() {
        respond(500, problem(500, "Internal server error", "The service cannot complete the request."));

        ApiException e = registerFails();

        assertEquals(ApiErrorKind.SERVER, e.kind());
        assertEquals(500, e.status());
    }

    @Test
    void internalErrorWithoutProblemBodyIsServer() {
        respond(500, "<html>Internal Server Error</html>");

        assertEquals(ApiErrorKind.SERVER, resendFails().kind());
    }

    @Test
    void serviceUnavailableIsUnavailable() {
        respond(503, problem(503, "Database error", "The service cannot complete the database operation."));

        assertEquals(ApiErrorKind.UNAVAILABLE, resendFails().kind());
    }

    @Test
    void rejectedJsonIsUnexpected() {
        respond(422, "Failed to deserialize the JSON body");

        assertEquals(ApiErrorKind.UNEXPECTED, registerFails().kind());
    }

    @Test
    void classificationTable() {
        assertEquals(ApiErrorKind.EMAIL_TAKEN, AuthApi.classify(AuthApi.Operation.REGISTER, 409));
        assertEquals(ApiErrorKind.UNEXPECTED, AuthApi.classify(AuthApi.Operation.VERIFY_EMAIL, 409));
        assertEquals(ApiErrorKind.UNAVAILABLE, AuthApi.classify(AuthApi.Operation.REGISTER, 502));
        assertEquals(ApiErrorKind.UNAVAILABLE, AuthApi.classify(AuthApi.Operation.REGISTER, 504));
        assertEquals(ApiErrorKind.UNEXPECTED, AuthApi.classify(AuthApi.Operation.REGISTER, 404));
        assertEquals(ApiErrorKind.UNEXPECTED, AuthApi.classify(AuthApi.Operation.REGISTER, 301));
    }

    // --- Fallas sin respuesta útil ---

    @Test
    void successWithUnreadableBodyIsUnexpected() {
        respond(201, "not json");

        assertEquals(ApiErrorKind.UNEXPECTED, registerFails().kind());
    }

    @Test
    void unreachableServiceIsNetworkError() {
        server.stop(0);

        ApiException e = registerFails();

        assertEquals(ApiErrorKind.NETWORK, e.kind());
        assertEquals(0, e.status());
    }

    @Test
    void slowServiceTimesOut() {
        server.createContext("/", exchange -> {
            try {
                release.await(5, TimeUnit.SECONDS);
            } catch (InterruptedException e) {
                Thread.currentThread().interrupt();
            }
            send(exchange, 201, ACCOUNT.formatted(false, true));
        });
        api = new AuthApi(new ApiConfig(baseUrl(), Duration.ofSeconds(1), Duration.ofMillis(200)));

        ApiException e = registerFails();

        assertEquals(ApiErrorKind.TIMEOUT, e.kind());
        assertEquals(0, e.status());
    }

    // --- RNF-SEG-04: la contraseña y el token no salen en textos ---

    @Test
    void passwordNeverAppearsInExceptions() {
        respond(400, problem(400, "Invalid request", "Invalid password format: " + PASSWORD));
        assertSecretAbsent(registerFails(), PASSWORD);

        respond(201, "{\"echo\":\"" + PASSWORD + "\"");
        assertSecretAbsent(registerFails(), PASSWORD);

        server.stop(0);
        assertSecretAbsent(registerFails(), PASSWORD);
    }

    @Test
    void tokenNeverAppearsInExceptions() {
        respond(400, problem(400, "Invalid request", "Invalid or expired verification token"));

        assertSecretAbsent(assertThrows(ApiException.class, () -> api.verifyEmail("tok-secret-42")), "tok-secret-42");
    }

    @Test
    void requestBodiesHideSecretsInToString() {
        String register = new AuthApi.RegisterRequest("ana@uv.mx", PASSWORD).toString();
        String verify = new AuthApi.VerifyEmailRequest("tok-secret-42").toString();

        assertFalse(register.contains(PASSWORD), register);
        assertTrue(register.contains("ana@uv.mx"), register);
        assertFalse(verify.contains("tok-secret-42"), verify);
    }

    // --- Utilidades ---

    private ApiException registerFails() {
        return assertThrows(ApiException.class, () -> api.register("ana@uv.mx", PASSWORD));
    }

    private ApiException resendFails() {
        return assertThrows(ApiException.class, (Executable) () -> api.resendVerification("ana@uv.mx"));
    }

    /** Revisa el mensaje, la cadena de causas y la traza completa. */
    private static void assertSecretAbsent(ApiException e, String secret) {
        assertNotNull(e.getMessage());
        StringWriter trace = new StringWriter();
        e.printStackTrace(new PrintWriter(trace));
        assertFalse(trace.toString().contains(secret), trace.toString());
        assertFalse(e.toString().contains(secret));
    }

    private URI baseUrl() {
        return URI.create("http://127.0.0.1:" + server.getAddress().getPort() + "/");
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
