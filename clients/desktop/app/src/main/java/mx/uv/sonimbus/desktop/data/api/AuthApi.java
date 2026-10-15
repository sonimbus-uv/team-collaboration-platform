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
import java.time.Duration;
import java.util.Map;

/**
 * Cliente HTTP del Auth Service. Las llamadas bloquean: deben hacerse fuera del hilo de JavaFX.
 */
public class AuthApi {

    private static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(5);
    private static final Duration REQUEST_TIMEOUT = Duration.ofSeconds(15);

    private final HttpClient http;
    private final String baseUrl;
    private final ObjectMapper json = new ObjectMapper()
            .setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
            .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

    public AuthApi(URI baseUrl) {
        this(HttpClient.newBuilder().connectTimeout(CONNECT_TIMEOUT).build(), baseUrl);
    }

    AuthApi(HttpClient http, URI baseUrl) {
        this.http = http;
        this.baseUrl = baseUrl.toString().replaceAll("/+$", "");
    }

    /** CU-01. Si {@code verificationEmailSent} es falso, hay que ofrecer el reenvío. */
    public Account register(String email, String password) throws ApiException {
        return read(post("/auth/register", Map.of("email", email, "password", password)));
    }

    /** CU-02. */
    public Account verifyEmail(String token) throws ApiException {
        return read(post("/auth/verify-email", Map.of("token", token)));
    }

    /**
     * CU-02. El servicio responde igual aunque la cuenta no exista, ya esté verificada o el
     * reenvío anterior tenga menos de 60 segundos.
     */
    public void resendVerification(String email) throws ApiException {
        post("/auth/resend-verification", Map.of("email", email));
    }

    private HttpResponse<String> post(String path, Map<String, String> body) throws ApiException {
        HttpRequest request;
        try {
            request = HttpRequest.newBuilder(URI.create(baseUrl + path))
                    .timeout(REQUEST_TIMEOUT)
                    .header("Content-Type", "application/json")
                    .header("Accept", "application/json")
                    .POST(HttpRequest.BodyPublishers.ofString(json.writeValueAsString(body)))
                    .build();
        } catch (JsonProcessingException e) {
            throw new ApiException(ApiError.UNEXPECTED, e);
        }

        HttpResponse<String> response;
        try {
            response = http.send(request, HttpResponse.BodyHandlers.ofString());
        } catch (IOException e) {
            throw new ApiException(ApiError.NETWORK, e);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new ApiException(ApiError.NETWORK, e);
        }

        if (response.statusCode() / 100 != 2) {
            throw toException(response);
        }
        return response;
    }

    private Account read(HttpResponse<String> response) throws ApiException {
        try {
            return json.readValue(response.body(), Account.class);
        } catch (JsonProcessingException e) {
            throw new ApiException(ApiError.UNEXPECTED, e);
        }
    }

    private ApiException toException(HttpResponse<String> response) {
        int status = response.statusCode();
        String detail = problemDetail(response.body());
        Duration retryAfter = response.headers().firstValue("Retry-After")
                .map(AuthApi::parseSeconds)
                .orElse(null);
        return new ApiException(classify(status, detail), status, detail, retryAfter);
    }

    /**
     * El servicio todavía envía {@code type: "about:blank"} en todos sus errores, así que los
     * casos de un 400 solo se distinguen por el texto de {@code detail}. Cuando el contrato
     * defina un {@code type} por caso, hay que decidir con él y quitar la comparación de textos.
     */
    private static ApiError classify(int status, String detail) {
        return switch (status) {
            case 400 -> switch (detail) {
                case "Invalid email format" -> ApiError.INVALID_EMAIL;
                case "Password is too short", "Password is too long", "Invalid password format" ->
                        ApiError.INVALID_PASSWORD;
                case "Invalid or expired verification token" -> ApiError.INVALID_TOKEN;
                default -> ApiError.INVALID_REQUEST;
            };
            case 409 -> ApiError.EMAIL_ALREADY_EXISTS;
            case 422 -> ApiError.INVALID_REQUEST;
            case 429 -> ApiError.TOO_MANY_REQUESTS;
            case 502, 503, 504 -> ApiError.SERVICE_UNAVAILABLE;
            default -> ApiError.UNEXPECTED;
        };
    }

    private String problemDetail(String body) {
        try {
            ProblemDetails problem = json.readValue(body, ProblemDetails.class);
            return problem != null && problem.detail() != null ? problem.detail() : "";
        } catch (JsonProcessingException e) {
            return "";
        }
    }

    private static Duration parseSeconds(String value) {
        try {
            return Duration.ofSeconds(Long.parseLong(value.strip()));
        } catch (NumberFormatException e) {
            return null;
        }
    }
}
