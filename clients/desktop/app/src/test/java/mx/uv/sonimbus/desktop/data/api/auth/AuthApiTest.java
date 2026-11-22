package mx.uv.sonimbus.desktop.data.api.auth;

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

import mx.uv.sonimbus.desktop.data.api.ApiConfig;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.registration.Account;
import mx.uv.sonimbus.desktop.data.api.auth.registration.RegistrationAttempt;
import mx.uv.sonimbus.desktop.data.api.auth.session.TokenResponse;
import mx.uv.sonimbus.desktop.data.logging.AuthLog;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.BeforeEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.function.Executable;

import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.time.ZonedDateTime;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import java.util.UUID;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.logging.Handler;
import java.util.logging.LogRecord;
import java.util.logging.Logger;

class AuthApiTest {

    private static final String PASSWORD = "ValidPass123!";
    private static final String ACCOUNT =
            """
            {"user_id":"7b0c","email":"ana@uv.mx","email_verified":%s,"status":"active",\
            "verification_email_sent":%s}""";

    private static final UUID ATTEMPT_ID = UUID.fromString("2f862636-cf2f-4c18-820e-7c29dc449ba0");
    private static final String ATTEMPT =
            """
            {"registration_attempt_id":"2f862636-cf2f-4c18-820e-7c29dc449ba0",
             "expires_at":"2030-01-01T00:10:00Z","verification_email_sent":%s}
            """;

    private HttpServer server;
    private AuthApi api;
    private final CountDownLatch release = new CountDownLatch(1);
    private String requestMethod;
    private String requestPath;
    private String requestContentType;
    private String requestBody;
    private String requestAuthorization;
    private String requestAttemptSecret;

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


    @Test
    void registerSendsCredentialsAndReadsPendingAttempt() throws ApiException, IOException {
        respond(201, ATTEMPT.formatted(true));

        RegistrationAttempt attempt = api.register("ana@uv.mx", PASSWORD);

        assertEquals("POST", requestMethod);
        assertEquals("/api/v1/auth/register", requestPath);
        assertEquals("application/json", requestContentType);
        assertEquals(Map.of("email", "ana@uv.mx", "password", PASSWORD), sentJson());
        assertEquals(ATTEMPT_ID, attempt.registrationAttemptId());
        assertEquals("2030-01-01T00:10:00Z", attempt.expiresAt());
        assertTrue(attempt.verificationEmailSent());
    }

    @Test
    void registerReportsEmailNotSent() throws ApiException {
        respond(201, ATTEMPT.formatted(false));

        assertFalse(api.register("ana@uv.mx", PASSWORD).verificationEmailSent());
    }

    @Test
    void unknownResponseFieldsAreIgnored() throws ApiException {
        respond(201, ATTEMPT.formatted(true).replace("}", ",\"display_name\":\"Ana\"}"));

        assertEquals(ATTEMPT_ID, api.register("ana@uv.mx", PASSWORD).registrationAttemptId());
    }

    @Test
    void verifyEmailSendsAttemptAndCodeAndReadsAccount() throws ApiException, IOException {
        respond(200, ACCOUNT.formatted(true, false));

        Account account = api.verifyEmail(ATTEMPT_ID, "00123456");

        assertEquals("/api/v1/auth/verify-email", requestPath);
        assertEquals(
                Map.of("registration_attempt_id", ATTEMPT_ID.toString(), "code", "00123456"),
                sentJson());
        assertTrue(account.emailVerified());
    }

    @Test
    void resendVerificationReadsAttempt() throws ApiException, IOException {
        respond(202, ATTEMPT.formatted(true));

        api.resendVerification(ATTEMPT_ID);

        assertEquals("/api/v1/auth/resend-verification", requestPath);
        assertEquals(Map.of("registration_attempt_id", ATTEMPT_ID.toString()), sentJson());
    }


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
        for (String detail :
                new String[] {"Invalid email format", "Password is too short", "Something new"}) {
            respond(400, problem(400, "Invalid request", detail));
            assertEquals(ApiErrorKind.VALIDATION, registerFails().kind(), detail);
        }
        respond(400, problem(400, "Invalid request", "Invalid email format"));
        assertEquals(ApiErrorKind.VALIDATION, resendFails().kind());
        respond(400, problem(400, "Invalid request", "Invalid or expired verification token"));
        assertEquals(
                ApiErrorKind.VALIDATION,
                assertThrows(ApiException.class, () -> api.verifyEmail(ATTEMPT_ID, "12345678"))
                        .kind());
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
        server.createContext(
                "/",
                exchange -> {
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
        String inTwoMinutes =
                DateTimeFormatter.RFC_1123_DATE_TIME.format(ZonedDateTime.now().plusMinutes(2));
        long seconds = AuthApi.parseRetryAfter(inTwoMinutes).toSeconds();
        assertTrue(seconds > 100 && seconds <= 120, "segundos: " + seconds);
        String past =
                DateTimeFormatter.RFC_1123_DATE_TIME.format(ZonedDateTime.now().minusMinutes(2));
        assertEquals(Duration.ZERO, AuthApi.parseRetryAfter(past));
    }

    @Test
    void internalErrorIsServer() {
        respond(
                500,
                problem(500, "Internal server error", "The service cannot complete the request."));

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
        respond(
                503,
                problem(
                        503,
                        "Database error",
                        "The service cannot complete the database operation."));

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
        assertEquals(
                ApiErrorKind.UNEXPECTED, AuthApi.classify(AuthApi.Operation.VERIFY_EMAIL, 409));
        assertEquals(ApiErrorKind.UNAVAILABLE, AuthApi.classify(AuthApi.Operation.REGISTER, 502));
        assertEquals(ApiErrorKind.UNAVAILABLE, AuthApi.classify(AuthApi.Operation.REGISTER, 504));
        assertEquals(ApiErrorKind.UNEXPECTED, AuthApi.classify(AuthApi.Operation.REGISTER, 404));
        assertEquals(ApiErrorKind.UNEXPECTED, AuthApi.classify(AuthApi.Operation.REGISTER, 301));
    }


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
        server.createContext(
                "/",
                exchange -> {
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

        assertSecretAbsent(
                assertThrows(
                        ApiException.class, () -> api.verifyEmail(ATTEMPT_ID, "tok-secret-42")),
                "tok-secret-42");
    }

    @Test
    void requestBodiesHideSecretsInToString() {
        String register = new AuthApi.RegisterRequest("ana@uv.mx", PASSWORD).toString();
        String verify = new AuthApi.VerifyEmailRequest(ATTEMPT_ID, "tok-secret-42").toString();

        assertFalse(register.contains(PASSWORD), register);
        assertFalse(register.contains("ana@uv.mx"), register);
        assertFalse(verify.contains("tok-secret-42"), verify);
    }

    @Test
    void legacyAccountResponseCannotBeUsedAsARegistrationAttempt() {
        respond(201, ACCOUNT.formatted(false, true));
        assertEquals(ApiErrorKind.UNEXPECTED, registerFails().kind());
    }

    @Test
    void nullResponseIsNotAcceptedAsAnAttempt() {
        respond(201, "null");
        assertEquals(ApiErrorKind.UNEXPECTED, registerFails().kind());
    }


    private ApiException registerFails() {
        return assertThrows(ApiException.class, () -> api.register("ana@uv.mx", PASSWORD));
    }

    private ApiException resendFails() {
        return assertThrows(
                ApiException.class, (Executable) () -> api.resendVerification(ATTEMPT_ID));
    }

    @Test
    void failedRequestsAreLoggedAndStillPropagateWithoutSensitiveDetails() {
        respond(503, "{\"detail\":\"private-server-data\"}");
        RecordingHandler handler = new RecordingHandler();
        Logger logger = Logger.getLogger(AuthLog.class.getName());
        logger.addHandler(handler);
        try {
            ApiException failure =
                    assertThrows(ApiException.class, () -> api.register("ana@uv.mx", PASSWORD));

            assertEquals(ApiErrorKind.UNAVAILABLE, failure.kind());
            assertEquals(503, failure.status());
            String events = String.join("\n", handler.messages);
            assertTrue(events.contains("event=request-started operation=register"));
            assertTrue(
                    events.contains(
                            "event=request-failed operation=register error_kind=UNAVAILABLE"
                                    + " http_status=503"));
            assertFalse(events.contains("request-succeeded"));
            assertFalse(events.contains(PASSWORD));
            assertFalse(events.contains("ana@uv.mx"));
            assertFalse(events.contains("private-server-data"));
        } finally {
            logger.removeHandler(handler);
        }
    }

    private static final class RecordingHandler extends Handler {
        private final List<String> messages = new ArrayList<>();

        @Override
        public void publish(LogRecord record) {
            assertNull(record.getThrown());
            messages.add(record.getMessage());
        }

        @Override
        public void flush() {}

        @Override
        public void close() {}
    }

    private static void assertSecretAbsent(ApiException failure, String secret) {
        assertNotNull(failure.getMessage());
        for (Throwable cause = failure; cause != null; cause = cause.getCause()) {
            assertFalse(cause.toString().contains(secret));
        }
    }

    private URI baseUrl() {
        return URI.create("http://127.0.0.1:" + server.getAddress().getPort() + "/");
    }

    private void respond(int status, String body) {
        try {
            server.removeContext("/");
        } catch (IllegalArgumentException e) {
        }
        server.createContext("/", exchange -> send(exchange, status, body));
    }

    private void send(HttpExchange exchange, int status, String body) throws IOException {
        requestAuthorization = exchange.getRequestHeaders().getFirst("Authorization");
        requestAttemptSecret = exchange.getRequestHeaders().getFirst("X-Auth-Attempt-Secret");
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
        return new ObjectMapper().readValue(requestBody, new TypeReference<>() {});
    }

    @Test
    void loginSendsDesktopCredentialsWithoutAnOptionalNullDeviceName() throws Exception {
        respond(200, tokenJson());
        TokenResponse response = api.login("ana@uv.mx", PASSWORD);
        assertEquals("/api/v1/auth/login", requestPath);
        assertEquals(
                Map.of("email", "ana@uv.mx", "password", PASSWORD, "client_type", "desktop"),
                sentJson());
        var tokens = response.toSessionTokens(java.time.Instant.now());
        assertEquals("private-refresh", tokens.refreshToken());
        assertEquals("private-access", tokens.access().value());
        assertFalse(response.toString().contains("private"));
    }

    @Test
    void refreshSendsOnlyTheRefreshCredential() throws Exception {
        respond(200, tokenJson());
        api.refresh("old-private-refresh");
        assertEquals("/api/v1/auth/refresh", requestPath);
        assertEquals(Map.of("refresh_token", "old-private-refresh"), sentJson());
    }

    @Test
    void logoutAcceptsOnlyConfirmed204AndSendsOnlyTheRefreshCredential() throws Exception {
        respond(204, "");
        api.logout("private-refresh");
        assertEquals("/api/v1/auth/logout", requestPath);
        assertEquals(Map.of("refresh_token", "private-refresh"), sentJson());
        respond(200, "{}");
        ApiException failure =
                assertThrows(ApiException.class, () -> api.logout("private-refresh"));
        assertEquals(ApiErrorKind.UNEXPECTED, failure.kind());
        assertSecretAbsent(failure, "private-refresh");
    }

    @Test
    void login401IsGenericAndNeverExposesProblemDetails() {
        respond(401, problem(401, "private internal reason", PASSWORD));
        ApiException failure =
                assertThrows(ApiException.class, () -> api.login("ana@uv.mx", PASSWORD));
        assertEquals(ApiErrorKind.INVALID_CREDENTIALS, failure.kind());
        assertSecretAbsent(failure, PASSWORD);
        assertFalse(failure.getMessage().contains("private internal reason"));
    }

    @Test
    void sessionRequestObjectsRedactCredentials() {
        assertFalse(
                new AuthApi.LoginRequest("ana@uv.mx", PASSWORD, "desktop")
                        .toString()
                        .contains(PASSWORD));
        assertFalse(new AuthApi.RefreshRequest("private-refresh").toString().contains("private"));
    }

    @Test
    void passwordRequestsUseExactContractsAndAuthorizationOnlyForChange() throws Exception {
        respond(
                202,
                "{\"recovery_attempt_id\":\""
                        + ATTEMPT_ID
                        + "\",\"expires_at\":\"2030-01-01T00:10:00Z\",\"message\":\"generic\"}");
        api.forgotPassword("ana@uv.mx");
        assertEquals("/api/v1/auth/forgot-password", requestPath);
        assertEquals(Map.of("email", "ana@uv.mx"), sentJson());
        assertNull(requestAuthorization);
        respond(204, "");
        api.resetPassword(ATTEMPT_ID, "00123456", PASSWORD);
        assertEquals("/api/v1/auth/reset-password", requestPath);
        assertEquals(
                Map.of(
                        "recovery_attempt_id",
                        ATTEMPT_ID.toString(),
                        "code",
                        "00123456",
                        "new_password",
                        PASSWORD),
                sentJson());
        assertNull(requestAuthorization);
        api.changePassword("private-access", "old", PASSWORD);
        assertEquals("/api/v1/auth/change-password", requestPath);
        assertEquals("Bearer private-access", requestAuthorization);
        assertEquals(Map.of("current_password", "old", "new_password", PASSWORD), sentJson());
    }

    @Test
    void passwordErrorsAcceptOnlyKnownProblemTypesWithoutDetailLeak() {
        respond(
                400,
                "{\"type\":\"/problems/invalid-current-password\",\"detail\":\""
                        + PASSWORD
                        + "\"}");
        ApiException failure =
                assertThrows(
                        ApiException.class, () -> api.changePassword("token", "old", PASSWORD));
        assertEquals("/problems/invalid-current-password", failure.problemType().orElseThrow());
        assertSecretAbsent(failure, PASSWORD);
        respond(400, "{\"type\":\"/problems/" + PASSWORD + "\",\"detail\":\"private\"}");
        assertTrue(
                assertThrows(ApiException.class, () -> api.changePassword("token", "old", PASSWORD))
                        .problemType()
                        .isEmpty());
        respond(400, "not json " + PASSWORD);
        assertTrue(
                assertThrows(ApiException.class, () -> api.changePassword("token", "old", PASSWORD))
                        .problemType()
                        .isEmpty());
    }

    @Test
    void passwordMutationsRejectUnexpectedSuccessStatusesAndRedactBodies() {
        respond(200, "{}");
        assertThrows(ApiException.class, () -> api.resetPassword(ATTEMPT_ID, "00123456", PASSWORD));
        assertThrows(ApiException.class, () -> api.changePassword("token", "old", PASSWORD));
        assertThrows(ApiException.class, () -> api.forgotPassword("ana@uv.mx"));
        assertFalse(
                new AuthApi.ResetPasswordBody(ATTEMPT_ID, "00123456", PASSWORD)
                        .toString()
                        .contains(PASSWORD));
        assertFalse(
                new AuthApi.ChangePasswordBody("old-secret", PASSWORD)
                        .toString()
                        .contains("old-secret"));
    }

    @Test
    void googleAttemptAndExchangeUseDesktopPkceAndSeparateSecretHeader() throws Exception {
        String secret = "A".repeat(43);
        respond(
                201,
                "{\"attempt_id\":\""
                        + ATTEMPT_ID
                        + "\",\"attempt_secret\":\""
                        + secret
                        + "\",\"authorization_url\":\"https://auth.invalid/api/v1/auth/google/authorize?ticket="
                        + secret
                        + "\",\"expires_at\":\"2030-01-01T00:10:00Z\"}");
        var attempt = api.createGoogleAttempt(secret);
        assertEquals(ATTEMPT_ID, attempt.toAttempt().attemptId());
        assertEquals("/api/v1/auth/google/attempts", requestPath);
        assertEquals(
                Map.of(
                        "client_type",
                        "desktop",
                        "redirect_target",
                        "desktop",
                        "code_challenge",
                        secret),
                sentJson());
        assertNull(requestAuthorization);
        assertNull(requestAttemptSecret);
        respond(200, tokenJson());
        api.exchangeGoogle(
                new mx.uv.sonimbus.desktop.domain.google.GoogleExchange(
                        ATTEMPT_ID, "private-attempt-secret", secret, secret));
        assertEquals("/api/v1/auth/google/exchange", requestPath);
        assertEquals("private-attempt-secret", requestAttemptSecret);
        assertNull(requestAuthorization);
        assertEquals(
                Map.of(
                        "attempt_id",
                        ATTEMPT_ID.toString(),
                        "completion_code",
                        secret,
                        "code_verifier",
                        secret),
                sentJson());
        assertFalse(requestBody.contains("private-attempt-secret"));
    }

    @Test
    void googleConflictIsWhitelistedAndProofBodiesAreRedacted() {
        respond(
                409,
                "{\"type\":\"/problems/account-link-required\",\"detail\":\"private-provider-data\"}");
        var proof =
                new mx.uv.sonimbus.desktop.domain.google.GoogleExchange(
                        ATTEMPT_ID, "secret", "completion", "verifier");
        ApiException failure = assertThrows(ApiException.class, () -> api.exchangeGoogle(proof));
        assertEquals("/problems/account-link-required", failure.problemType().orElseThrow());
        assertFalse(failure.getMessage().contains("private-provider"));
        assertFalse(
                new AuthApi.GoogleExchangeBody(ATTEMPT_ID, "completion", "verifier")
                        .toString()
                        .contains("verifier"));
        assertFalse(
                new AuthApi.RequestAuthorization("X-Auth-Attempt-Secret", "private")
                        .toString()
                        .contains("private"));
    }

    @Test
    void googleLinkAndReauthenticationUseSessionButExchangeUsesOnlyAttemptSecret()
            throws Exception {
        String secret = "A".repeat(43);
        respond(
                201,
                "{\"attempt_id\":\""
                        + ATTEMPT_ID
                        + "\",\"attempt_secret\":\""
                        + secret
                        + "\",\"authorization_url\":\"https://auth.invalid/api/v1/auth/google/authorize?ticket="
                        + secret
                        + "\",\"expires_at\":\"2030-01-01T00:10:00Z\"}");
        api.createGoogleLinkAttempt("session-access", secret);
        assertEquals("/api/v1/auth/google/link-attempts", requestPath);
        assertEquals("Bearer session-access", requestAuthorization);
        assertEquals(Map.of("redirect_target", "desktop", "code_challenge", secret), sentJson());
        respond(204, "");
        api.reauthenticateGoogle("session-access", PASSWORD);
        assertEquals("/api/v1/auth/reauthenticate", requestPath);
        assertEquals(Map.of("password", PASSWORD), sentJson());
        api.exchangeGoogleLink(
                new mx.uv.sonimbus.desktop.domain.google.GoogleExchange(
                        ATTEMPT_ID, "attempt-private", secret, secret));
        assertEquals("/api/v1/auth/google/exchange", requestPath);
        assertNull(requestAuthorization);
        assertEquals("attempt-private", requestAttemptSecret);
        assertFalse(requestBody.contains("attempt-private"));
        assertFalse(new AuthApi.GoogleReauthenticateBody(PASSWORD).toString().contains(PASSWORD));
    }

    @Test
    void googleConnectionsUsesGetAndLinkExchangeRejectsLoginTokenResponse() throws Exception {
        respond(200, "{\"connections\":[]}");
        assertTrue(api.googleConnections("session-access").toConnection().isEmpty());
        assertEquals("GET", requestMethod);
        assertEquals("/api/v1/auth/connections", requestPath);
        assertEquals("Bearer session-access", requestAuthorization);
        assertTrue(requestBody.isEmpty());
        respond(200, tokenJson());
        assertThrows(
                ApiException.class,
                () ->
                        api.exchangeGoogleLink(
                                new mx.uv.sonimbus.desktop.domain.google.GoogleExchange(
                                        ATTEMPT_ID, "secret", "code", "verifier")));
    }

    private static String tokenJson() {
        return """
               {"access_token":"private-access","refresh_token":"private-refresh",
                "token_type":"Bearer","access_expires_in":900,
                "session_expires_at":"2030-01-01T00:00:00Z",
                "sid":"2f862636-cf2f-4c18-820e-7c29dc449ba0",
                "account_id":"3f862636-cf2f-4c18-820e-7c29dc449ba0"}
               """;
    }
}
