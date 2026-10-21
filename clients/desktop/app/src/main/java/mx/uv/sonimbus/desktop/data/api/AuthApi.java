package mx.uv.sonimbus.desktop.data.api;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
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

/**
 * Cliente HTTP del Auth Service. Las llamadas bloquean: deben hacerse fuera del hilo de JavaFX
 * (ver {@code ui.common.BackgroundTasks}).
 */
public class AuthApi {

    /** Operaciones del servicio; el mismo código HTTP puede significar cosas distintas en cada una. */
    enum Operation {
        REGISTER("register", AuthPaths.REGISTER),
        VERIFY_EMAIL("verify-email", AuthPaths.VERIFY_EMAIL),
        RESEND_VERIFICATION("resend-verification", AuthPaths.RESEND_VERIFICATION);

        private final String label;
        private final String path;

        Operation(String label, String path) {
            this.label = label;
            this.path = path;
        }
    }

    /** Cuerpo de {@code POST /auth/register}. {@link #toString()} oculta la contraseña. */
    record RegisterRequest(String email, String password) {
        @Override
        public String toString() {
            return "RegisterRequest[email=" + email + ", password=[REDACTED]]";
        }
    }

    /** Cuerpo de {@code POST /auth/verify-email}. {@link #toString()} oculta el token. */
    record VerifyEmailRequest(String token) {
        @Override
        public String toString() {
            return "VerifyEmailRequest[token=[REDACTED]]";
        }
    }

    record ResendVerificationRequest(String email) {
    }

    private final HttpClient http;
    private final String baseUrl;
    private final Duration requestTimeout;
    private final ObjectMapper json = new ObjectMapper()
            .setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
            .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

    public AuthApi(ApiConfig config) {
        this(HttpClient.newBuilder().connectTimeout(config.connectTimeout()).build(), config);
    }

    AuthApi(HttpClient http, ApiConfig config) {
        this.http = http;
        this.baseUrl = config.authBaseUrl().toString().replaceAll("/+$", "");
        this.requestTimeout = config.requestTimeout();
    }

    /** CU-01. Si {@code verificationEmailSent} es falso, hay que ofrecer el reenvío. */
    public Account register(String email, String password) throws ApiException {
        return read(Operation.REGISTER, post(Operation.REGISTER, new RegisterRequest(email, password)));
    }

    /** CU-02. El escritorio todavía no lo usa: la verificación se hace con el enlace del correo. */
    public Account verifyEmail(String token) throws ApiException {
        return read(Operation.VERIFY_EMAIL, post(Operation.VERIFY_EMAIL, new VerifyEmailRequest(token)));
    }

    /**
     * CU-02. El servicio responde igual aunque la cuenta no exista, ya esté verificada o el
     * reenvío anterior tenga menos de 60 segundos.
     */
    public void resendVerification(String email) throws ApiException {
        post(Operation.RESEND_VERIFICATION, new ResendVerificationRequest(email));
    }

    private HttpResponse<String> post(Operation operation, Object body) throws ApiException {
        HttpRequest request;
        try {
            request = HttpRequest.newBuilder(URI.create(baseUrl + operation.path))
                    .timeout(requestTimeout)
                    .header("Content-Type", "application/json")
                    .header("Accept", "application/json")
                    .POST(HttpRequest.BodyPublishers.ofString(json.writeValueAsString(body)))
                    .build();
        } catch (JsonProcessingException e) {
            // Sin la causa: el mensaje de Jackson podría citar el cuerpo con la contraseña.
            throw new ApiException(ApiErrorKind.UNEXPECTED, operation.label, null);
        }

        HttpResponse<String> response;
        try {
            response = http.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (HttpTimeoutException e) {
            throw new ApiException(ApiErrorKind.TIMEOUT, operation.label, e);
        } catch (IOException e) {
            throw new ApiException(ApiErrorKind.NETWORK, operation.label, e);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new ApiException(ApiErrorKind.NETWORK, operation.label, e);
        }

        int status = response.statusCode();
        if (status / 100 != 2) {
            Duration retryAfter = response.headers().firstValue("Retry-After")
                    .map(AuthApi::parseRetryAfter)
                    .orElse(null);
            throw new ApiException(classify(operation, status), operation.label, status, retryAfter);
        }
        return response;
    }

    private Account read(Operation operation, HttpResponse<String> response) throws ApiException {
        try {
            return json.readValue(response.body(), Account.class);
        } catch (JsonProcessingException e) {
            throw new ApiException(ApiErrorKind.UNEXPECTED, operation.label, e);
        }
    }

    /**
     * Decide el caso solo con el código HTTP y la operación. El servicio envía
     * {@code type: "about:blank"} en todos sus errores y su {@code detail} no es contrato (#14).
     */
    static ApiErrorKind classify(Operation operation, int status) {
        return switch (status) {
            case 400 -> ApiErrorKind.VALIDATION;
            case 409 -> operation == Operation.REGISTER ? ApiErrorKind.EMAIL_TAKEN : ApiErrorKind.UNEXPECTED;
            case 429 -> ApiErrorKind.RATE_LIMITED;
            case 500 -> ApiErrorKind.SERVER;
            case 502, 503, 504 -> ApiErrorKind.UNAVAILABLE;
            // 422 y 415 los produce Axum cuando el JSON enviado no es válido: es un error del cliente.
            default -> ApiErrorKind.UNEXPECTED;
        };
    }

    /** {@code Retry-After} en segundos o como fecha HTTP (RFC 9110); {@code null} si no se entiende. */
    static Duration parseRetryAfter(String value) {
        String text = value.strip();
        try {
            return Duration.ofSeconds(Math.max(0, Long.parseLong(text)));
        } catch (NumberFormatException e) {
            // No son segundos: puede ser una fecha.
        }
        try {
            Duration wait = Duration.between(ZonedDateTime.now(),
                    ZonedDateTime.parse(text, DateTimeFormatter.RFC_1123_DATE_TIME));
            return wait.isNegative() ? Duration.ZERO : wait;
        } catch (DateTimeParseException e) {
            return null;
        }
    }
}
