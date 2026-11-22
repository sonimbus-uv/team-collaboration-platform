package mx.uv.sonimbus.desktop.data.api.auth;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;

import mx.uv.sonimbus.desktop.data.api.ApiConfig;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.google.GoogleAttemptResponse;
import mx.uv.sonimbus.desktop.data.api.auth.google.GoogleConnectionsResponse;
import mx.uv.sonimbus.desktop.data.api.auth.password.RecoveryResponse;
import mx.uv.sonimbus.desktop.data.api.auth.registration.Account;
import mx.uv.sonimbus.desktop.data.api.auth.registration.RegistrationAttempt;
import mx.uv.sonimbus.desktop.data.api.auth.session.TokenResponse;
import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.google.GoogleExchange;

import java.io.IOException;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.http.HttpTimeoutException;
import java.time.Duration;
import java.time.ZonedDateTime;
import java.time.format.DateTimeFormatter;
import java.time.format.DateTimeParseException;
import java.util.Set;
import java.util.UUID;

public class AuthApi {

    enum Operation {
        REGISTER("register", AuthPaths.REGISTER),
        VERIFY_EMAIL("verify-email", AuthPaths.VERIFY_EMAIL),
        RESEND_VERIFICATION("resend-verification", AuthPaths.RESEND_VERIFICATION),
        LOGIN("login", AuthPaths.LOGIN),
        REFRESH("refresh", AuthPaths.REFRESH),
        LOGOUT("logout", AuthPaths.LOGOUT),
        FORGOT_PASSWORD("forgot-password", AuthPaths.FORGOT_PASSWORD),
        RESET_PASSWORD("reset-password", AuthPaths.RESET_PASSWORD),
        CHANGE_PASSWORD("change-password", AuthPaths.CHANGE_PASSWORD),
        GOOGLE_ATTEMPT("google-attempt", AuthPaths.GOOGLE_ATTEMPT),
        GOOGLE_EXCHANGE("google-exchange", AuthPaths.GOOGLE_EXCHANGE),
        GOOGLE_LINK("google-link", AuthPaths.GOOGLE_LINK),
        GOOGLE_LINK_EXCHANGE("google-link-exchange", AuthPaths.GOOGLE_EXCHANGE),
        GOOGLE_REAUTHENTICATE("google-reauthenticate", AuthPaths.GOOGLE_REAUTHENTICATE),
        GOOGLE_CONNECTIONS("google-connections", AuthPaths.GOOGLE_CONNECTIONS);

        private final String label;
        private final String path;

        Operation(String label, String path) {
            this.label = label;
            this.path = path;
        }
    }

    record RegisterRequest(String email, String password) {
        @Override
        public String toString() {
            return "RegisterRequest[email=[REDACTED], password=[REDACTED]]";
        }
    }

    record VerifyEmailRequest(UUID registrationAttemptId, String code) {
        @Override
        public String toString() {
            return "VerifyEmailRequest[code=[REDACTED]]";
        }
    }

    record ResendVerificationRequest(UUID registrationAttemptId) {}

    record LoginRequest(String email, String password, String clientType) {
        @Override
        public String toString() {
            return "LoginRequest[REDACTED]";
        }
    }

    record RefreshRequest(String refreshToken) {
        @Override
        public String toString() {
            return "RefreshRequest[REDACTED]";
        }
    }

    record ForgotPasswordRequest(String email) {
        @Override
        public String toString() {
            return "ForgotPasswordRequest[REDACTED]";
        }
    }

    record ResetPasswordBody(UUID recoveryAttemptId, String code, String newPassword) {
        @Override
        public String toString() {
            return "ResetPasswordBody[REDACTED]";
        }
    }

    record ChangePasswordBody(String currentPassword, String newPassword) {
        @Override
        public String toString() {
            return "ChangePasswordBody[REDACTED]";
        }
    }

    record GoogleAttemptBody(String clientType, String redirectTarget, String codeChallenge) {}

    record GoogleLinkBody(String redirectTarget, String codeChallenge) {}

    record GoogleReauthenticateBody(String password) {
        @Override
        public String toString() {
            return "GoogleReauthenticateBody[REDACTED]";
        }
    }

    record GoogleExchangeBody(UUID attemptId, String completionCode, String codeVerifier) {
        @Override
        public String toString() {
            return "GoogleExchangeBody[REDACTED]";
        }
    }

    record RequestAuthorization(String name, String value) {
        @Override
        public String toString() {
            return "RequestAuthorization[REDACTED]";
        }
    }

    private static final Set<String> AUTH_PROBLEMS =
            Set.of(
                    "/problems/invalid-current-password",
                    "/problems/invalid-recovery-code",
                    "/problems/invalid-password",
                    "/problems/malformed-request",
                    "/problems/password-unchanged",
                    "/problems/password-change-not-allowed",
                    "/problems/credentials-changed",
                    "/problems/account-link-required",
                    "/problems/reauthentication-required",
                    "/problems/identity-already-linked",
                    "/problems/provider-already-linked",
                    "/problems/account-link-conflict",
                    "/problems/invalid-credentials",
                    "/problems/invalid-access-token");

    private final HttpClient httpClient;
    private final String baseUrl;
    private final Duration requestTimeout;
    private final ObjectMapper objectMapper =
            new ObjectMapper()
                    .setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
                    .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

    public AuthApi(ApiConfig config) {
        this(HttpClient.newBuilder().connectTimeout(config.connectTimeout()).build(), config);
    }

    AuthApi(HttpClient httpClient, ApiConfig config) {
        this.httpClient = httpClient;
        this.baseUrl = config.authBaseUrl().toString().replaceAll("/+$", "");
        this.requestTimeout = config.requestTimeout();
    }

    public RegistrationAttempt register(String email, String password) throws ApiException {
        return execute(
                Operation.REGISTER,
                new RegisterRequest(email, password),
                RegistrationAttempt.class);
    }

    public Account verifyEmail(UUID attemptId, String code) throws ApiException {
        return execute(
                Operation.VERIFY_EMAIL, new VerifyEmailRequest(attemptId, code), Account.class);
    }

    public RegistrationAttempt resendVerification(UUID attemptId) throws ApiException {
        return execute(
                Operation.RESEND_VERIFICATION,
                new ResendVerificationRequest(attemptId),
                RegistrationAttempt.class);
    }

    public TokenResponse login(String email, String password) throws ApiException {
        return execute(
                Operation.LOGIN, new LoginRequest(email, password, "desktop"), TokenResponse.class);
    }

    public TokenResponse refresh(String refreshToken) throws ApiException {
        return execute(Operation.REFRESH, new RefreshRequest(refreshToken), TokenResponse.class);
    }

    public void logout(String refreshToken) throws ApiException {
        execute(Operation.LOGOUT, new RefreshRequest(refreshToken), Void.class);
    }

    public RecoveryResponse forgotPassword(String email) throws ApiException {
        return execute(
                Operation.FORGOT_PASSWORD,
                new ForgotPasswordRequest(email),
                RecoveryResponse.class);
    }

    public void resetPassword(UUID attemptId, String code, String newPassword) throws ApiException {
        execute(
                Operation.RESET_PASSWORD,
                new ResetPasswordBody(attemptId, code, newPassword),
                Void.class);
    }

    public void changePassword(String accessToken, String currentPassword, String newPassword)
            throws ApiException {
        execute(
                Operation.CHANGE_PASSWORD,
                new ChangePasswordBody(currentPassword, newPassword),
                Void.class,
                new RequestAuthorization("Authorization", "Bearer " + accessToken));
    }

    public GoogleAttemptResponse createGoogleAttempt(String challenge) throws ApiException {
        String target = googleTarget();
        return execute(
                Operation.GOOGLE_ATTEMPT,
                new GoogleAttemptBody("desktop", target, challenge),
                GoogleAttemptResponse.class);
    }

    public TokenResponse exchangeGoogle(GoogleExchange proof) throws ApiException {
        return execute(
                Operation.GOOGLE_EXCHANGE,
                new GoogleExchangeBody(
                        proof.attemptId(), proof.completionCode(), proof.codeVerifier()),
                TokenResponse.class,
                new RequestAuthorization("X-Auth-Attempt-Secret", proof.attemptSecret()));
    }

    public GoogleAttemptResponse createGoogleLinkAttempt(String accessToken, String challenge)
            throws ApiException {
        return execute(
                Operation.GOOGLE_LINK,
                new GoogleLinkBody(googleTarget(), challenge),
                GoogleAttemptResponse.class,
                new RequestAuthorization("Authorization", "Bearer " + accessToken));
    }

    public void exchangeGoogleLink(GoogleExchange proof) throws ApiException {
        execute(
                Operation.GOOGLE_LINK_EXCHANGE,
                new GoogleExchangeBody(
                        proof.attemptId(), proof.completionCode(), proof.codeVerifier()),
                Void.class,
                new RequestAuthorization("X-Auth-Attempt-Secret", proof.attemptSecret()));
    }

    public void reauthenticateGoogle(String accessToken, String password) throws ApiException {
        execute(
                Operation.GOOGLE_REAUTHENTICATE,
                new GoogleReauthenticateBody(password),
                Void.class,
                new RequestAuthorization("Authorization", "Bearer " + accessToken));
    }

    public GoogleConnectionsResponse googleConnections(String accessToken) throws ApiException {
        return execute(
                Operation.GOOGLE_CONNECTIONS,
                null,
                GoogleConnectionsResponse.class,
                new RequestAuthorization("Authorization", "Bearer " + accessToken));
    }

    private String googleTarget() throws ApiException {
        String target =
                System.getProperty(
                        "sonimbus.auth.google.target",
                        System.getenv().getOrDefault("SONIMBUS_AUTH_GOOGLE_TARGET", "desktop"));
        if (!target.matches("[A-Za-z0-9_-]{1,64}")) {
            throw new ApiException(ApiErrorKind.VALIDATION, "google-attempt", null);
        }
        return target;
    }

    private <T> T execute(Operation operation, Object requestBody, Class<T> responseType)
            throws ApiException {

        return execute(operation, requestBody, responseType, null);
    }

    private <T> T execute(
            Operation operation,
            Object requestBody,
            Class<T> responseType,
            RequestAuthorization authorization)
            throws ApiException {

        AuthLog.requestStarted(operation.label);

        try {

            HttpRequest request = createRequest(operation, requestBody, authorization);
            HttpResponse<String> response = sendRequest(operation, request);

            validateStatus(operation, response);

            T responseBody = readResponse(operation, response, responseType);
            AuthLog.requestSucceeded(operation.label, response.statusCode());

            return responseBody;

        } catch (ApiException failure) {
            AuthLog.requestFailed(operation.label, failure);
            throw failure;
        }
    }

    private HttpRequest createRequest(
            Operation operation, Object requestBody, RequestAuthorization authorization)
            throws ApiException {

        try {

            HttpRequest.Builder builder =
                    HttpRequest.newBuilder(URI.create(baseUrl + operation.path))
                            .timeout(requestTimeout)
                            .header("Accept", "application/json");
            if (operation == Operation.GOOGLE_CONNECTIONS) {
                builder.GET();
            } else {
                builder.header("Content-Type", "application/json")
                        .POST(
                                HttpRequest.BodyPublishers.ofString(
                                        objectMapper.writeValueAsString(requestBody)));
            }
            if (authorization != null) {
                builder.header(authorization.name(), authorization.value());
            }
            return builder.build();

        } catch (JsonProcessingException serializationFailure) {
            throw new ApiException(ApiErrorKind.UNEXPECTED, operation.label, null);
        }
    }

    private HttpResponse<String> sendRequest(Operation operation, HttpRequest request)
            throws ApiException {

        try {

            return httpClient.send(request, HttpResponse.BodyHandlers.ofString());

        } catch (HttpTimeoutException timeoutFailure) {
            throw new ApiException(ApiErrorKind.TIMEOUT, operation.label, timeoutFailure);
        } catch (IOException connectionFailure) {
            throw new ApiException(ApiErrorKind.NETWORK, operation.label, connectionFailure);
        } catch (InterruptedException interruption) {
            Thread.currentThread().interrupt();
            throw new ApiException(ApiErrorKind.NETWORK, operation.label, interruption);
        }
    }

    private void validateStatus(Operation operation, HttpResponse<String> response)
            throws ApiException {

        int statusCode = response.statusCode();

        if (statusCode / 100 != 2) {
            Duration retryAfter =
                    response.headers()
                            .firstValue("Retry-After")
                            .map(AuthApi::parseRetryAfter)
                            .orElse(null);

            throw new ApiException(
                    new ApiException.ResponseDetails(
                            statusCode, retryAfter, recognizedProblem(operation, response)),
                    classify(operation, statusCode),
                    operation.label);
        }
        int expected =
                switch (operation) {
                    case REGISTER, GOOGLE_ATTEMPT, GOOGLE_LINK -> 201;
                    case RESEND_VERIFICATION, FORGOT_PASSWORD -> 202;
                    case LOGOUT,
                                    RESET_PASSWORD,
                                    CHANGE_PASSWORD,
                                    GOOGLE_LINK_EXCHANGE,
                                    GOOGLE_REAUTHENTICATE ->
                            204;
                    default -> 200;
                };
        if (statusCode != expected) {
            throw new ApiException(ApiErrorKind.UNEXPECTED, operation.label, statusCode, null);
        }
    }

    private String recognizedProblem(Operation operation, HttpResponse<String> response) {
        if (operation != Operation.CHANGE_PASSWORD
                && operation != Operation.RESET_PASSWORD
                && operation != Operation.FORGOT_PASSWORD
                && operation != Operation.GOOGLE_EXCHANGE
                && operation != Operation.GOOGLE_LINK
                && operation != Operation.GOOGLE_LINK_EXCHANGE
                && operation != Operation.GOOGLE_REAUTHENTICATE
                && operation != Operation.GOOGLE_CONNECTIONS) {
            return null;
        }
        try {
            var problem = objectMapper.readTree(response.body());
            String type = problem == null ? "" : problem.path("type").asText();
            return AUTH_PROBLEMS.contains(type) ? type : null;
        } catch (JsonProcessingException invalidProblem) {
            return null;
        }
    }

    private <T> T readResponse(
            Operation operation, HttpResponse<String> response, Class<T> responseType)
            throws ApiException {

        if (responseType == Void.class && response.body().isBlank()) {
            return null;
        }

        try {

            T responseBody = objectMapper.readValue(response.body(), responseType);

            if (responseBody == null) {
                throw new ApiException(ApiErrorKind.UNEXPECTED, operation.label, null);
            }

            return responseBody;

        } catch (JsonProcessingException deserializationFailure) {
            throw new ApiException(ApiErrorKind.UNEXPECTED, operation.label, null);
        }
    }

    static ApiErrorKind classify(Operation operation, int statusCode) {
        return switch (statusCode) {
            case 400 -> ApiErrorKind.VALIDATION;
            case 401 ->
                    operation == Operation.LOGIN
                            ? ApiErrorKind.INVALID_CREDENTIALS
                            : ApiErrorKind.UNEXPECTED;
            case 409 ->
                    operation == Operation.REGISTER
                            ? ApiErrorKind.EMAIL_TAKEN
                            : ApiErrorKind.UNEXPECTED;
            case 429 -> ApiErrorKind.RATE_LIMITED;
            case 500 -> ApiErrorKind.SERVER;
            case 502, 503, 504 -> ApiErrorKind.UNAVAILABLE;
            default -> ApiErrorKind.UNEXPECTED;
        };
    }

    static Duration parseRetryAfter(String value) {

        String retryAfterValue = value.strip();

        try {

            return Duration.ofSeconds(Math.max(0, Long.parseLong(retryAfterValue)));

        } catch (NumberFormatException invalidSeconds) {
            return parseRetryAfterDate(retryAfterValue);
        }
    }

    private static Duration parseRetryAfterDate(String value) {

        try {
            Duration remainingWait =
                    Duration.between(
                            ZonedDateTime.now(),
                            ZonedDateTime.parse(value, DateTimeFormatter.RFC_1123_DATE_TIME));

            return remainingWait.isNegative() ? Duration.ZERO : remainingWait;

        } catch (DateTimeParseException invalidDate) {
            return null;
        }
    }
}
