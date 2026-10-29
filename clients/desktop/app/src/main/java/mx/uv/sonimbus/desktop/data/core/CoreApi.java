package mx.uv.sonimbus.desktop.data.core;

import com.fasterxml.jackson.annotation.JsonInclude;
import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import com.fasterxml.jackson.databind.type.CollectionType;
import java.io.IOException;
import java.net.URI;
import java.net.URLEncoder;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.net.http.HttpTimeoutException;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.List;
import java.util.Map;
import java.util.Optional;
import mx.uv.sonimbus.desktop.data.api.ApiConfig;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.session.Session;

/**
 * Cliente HTTP del Core Service ({@code contracts/openapi/core.yaml}). Envía el access token de
 * {@link Session} y, si Core lo rechaza, lo renueva una vez y repite la petición. Las llamadas
 * bloquean: deben hacerse fuera del hilo de JavaFX.
 */
public class CoreApi {

    static final String PREFIX = "/api/v1";

    private record CreateGroupRequest(String name, String description) {
    }

    private record CreateChannelRequest(String name, String type) {
    }

    /** {@link #toString()} oculta el código: quien lo tiene puede unirse al grupo. */
    private record JoinRequest(String code) {
        @Override
        public String toString() {
            return "JoinRequest[code=[REDACTED]]";
        }
    }

    private record ProfileRequest(String displayName) {
    }

    private final HttpClient http;
    private final String baseUrl;
    private final Duration requestTimeout;
    private final Session session;
    private final ObjectMapper json = new ObjectMapper()
            .setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
            .setSerializationInclusion(JsonInclude.Include.NON_NULL)
            .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

    public CoreApi(ApiConfig config, Session session) {
        this(HttpClient.newBuilder().connectTimeout(config.connectTimeout()).build(), config, session);
    }

    CoreApi(HttpClient http, ApiConfig config, Session session) {
        this.http = http;
        this.baseUrl = config.coreBaseUrl().toString().replaceAll("/+$", "");
        this.requestTimeout = config.requestTimeout();
        this.session = session;
    }

    /** Grupos del usuario, por nombre. */
    public List<Group> listGroups() throws ApiException {
        return list("list-groups", send("list-groups", "GET", "/groups", null), Group.class);
    }

    /** CU-08: el usuario queda como administrador. */
    public Group createGroup(String name, String description) throws ApiException {
        return data("create-group", send("create-group", "POST", "/groups",
                new CreateGroupRequest(name, description)), Group.class);
    }

    /** CU-10 con los valores por omisión de Core: caduca en 7 días y sin límite de usos. */
    public Invitation createInvitation(String groupId) throws ApiException {
        return data("create-invitation", send("create-invitation", "POST",
                "/groups/" + segment(groupId) + "/invitations", Map.of()), Invitation.class);
    }

    /** CU-11. Si el usuario ya era miembro, devuelve el grupo sin descontar un uso. */
    public Group joinGroup(String code) throws ApiException {
        return data("join-group", send("join-group", "POST", "/invitations/join", new JoinRequest(code)),
                Group.class);
    }

    /** Canales que el rol del usuario puede ver, por nombre. */
    public List<Channel> listChannels(String groupId) throws ApiException {
        return list("list-channels", send("list-channels", "GET",
                "/groups/" + segment(groupId) + "/channels", null), Channel.class);
    }

    /** CU-13. {@code name}: minúsculas, dígitos y guiones. */
    public Channel createChannel(String groupId, String name, String type) throws ApiException {
        return data("create-channel", send("create-channel", "POST",
                "/groups/" + segment(groupId) + "/channels", new CreateChannelRequest(name, type)), Channel.class);
    }

    /** CU-20: página del historial anterior a {@code before} (o la más reciente si es nulo). */
    public MessagePage listMessages(String channelId, String before, int limit) throws ApiException {
        String query = "?limit=" + limit + (before == null ? "" : "&before=" + segment(before));
        HttpResponse<String> response = send("list-messages", "GET",
                "/channels/" + segment(channelId) + "/messages" + query, null);
        return parse("list-messages", response.body(), MessagePage.class);
    }

    /** Perfil del usuario; vacío si todavía no guardó su nombre visible. */
    public Optional<Profile> myProfile() throws ApiException {
        try {
            return Optional.of(data("get-profile", send("get-profile", "GET", "/me/profile", null), Profile.class));
        } catch (ApiException e) {
            if (e.kind() == ApiErrorKind.NOT_FOUND) {
                return Optional.empty();
            }
            throw e;
        }
    }

    /** Guarda el nombre visible (D-29). */
    public Profile saveDisplayName(String displayName) throws ApiException {
        return data("put-profile", send("put-profile", "PUT", "/me/profile", new ProfileRequest(displayName)),
                Profile.class);
    }

    private HttpResponse<String> send(String operation, String method, String path, Object body)
            throws ApiException {
        String token = session.accessToken();
        HttpResponse<String> response = execute(operation, method, path, body, token);
        if (response.statusCode() == 401) {
            response = execute(operation, method, path, body, session.renewAfterRejection(token));
        }
        int status = response.statusCode();
        if (status / 100 != 2) {
            Duration retryAfter = response.headers().firstValue("Retry-After")
                    .map(CoreApi::parseSeconds)
                    .orElse(null);
            throw new ApiException(classify(status, problemType(response.body())), operation, status, retryAfter);
        }
        return response;
    }

    private HttpResponse<String> execute(String operation, String method, String path, Object body, String token)
            throws ApiException {
        HttpRequest.BodyPublisher publisher;
        try {
            publisher = body == null
                    ? HttpRequest.BodyPublishers.noBody()
                    : HttpRequest.BodyPublishers.ofString(json.writeValueAsString(body));
        } catch (JsonProcessingException e) {
            throw new ApiException(ApiErrorKind.UNEXPECTED, operation, null);
        }
        HttpRequest.Builder request = HttpRequest.newBuilder(URI.create(baseUrl + PREFIX + path))
                .timeout(requestTimeout)
                .header("Accept", "application/json")
                .header("Authorization", "Bearer " + token)
                .method(method, publisher);
        if (body != null) {
            request.header("Content-Type", "application/json");
        }
        try {
            return http.send(request.build(), HttpResponse.BodyHandlers.ofString());
        } catch (HttpTimeoutException e) {
            throw new ApiException(ApiErrorKind.TIMEOUT, operation, e);
        } catch (IOException e) {
            throw new ApiException(ApiErrorKind.NETWORK, operation, e);
        } catch (InterruptedException e) {
            Thread.currentThread().interrupt();
            throw new ApiException(ApiErrorKind.NETWORK, operation, e);
        }
    }

    /**
     * Core sí distingue sus errores por {@code type} (Problem Details, {@code /problems/<nombre>}),
     * que es parte del contrato; el código HTTP decide cuando el tipo no aporta.
     */
    static ApiErrorKind classify(int status, String type) {
        return switch (type == null ? "" : type) {
            case "/problems/invitation-invalid" -> ApiErrorKind.INVALID_INVITATION;
            case "/problems/banned" -> ApiErrorKind.BANNED;
            default -> switch (status) {
                case 400, 422 -> ApiErrorKind.VALIDATION;
                case 401 -> ApiErrorKind.UNAUTHORIZED;
                case 403 -> ApiErrorKind.FORBIDDEN;
                case 404 -> ApiErrorKind.NOT_FOUND;
                case 409 -> ApiErrorKind.CONFLICT;
                case 429 -> ApiErrorKind.RATE_LIMITED;
                case 500 -> ApiErrorKind.SERVER;
                case 502, 503, 504 -> ApiErrorKind.UNAVAILABLE;
                default -> ApiErrorKind.UNEXPECTED;
            };
        };
    }

    private String problemType(String body) {
        try {
            JsonNode type = json.readTree(body).path("type");
            return type.isTextual() ? type.asText() : null;
        } catch (JsonProcessingException | IllegalArgumentException e) {
            return null;
        }
    }

    private <T> T data(String operation, HttpResponse<String> response, Class<T> type) throws ApiException {
        try {
            return json.treeToValue(json.readTree(response.body()).path("data"), type);
        } catch (JsonProcessingException | IllegalArgumentException e) {
            throw new ApiException(ApiErrorKind.UNEXPECTED, operation, null);
        }
    }

    private <T> List<T> list(String operation, HttpResponse<String> response, Class<T> type) throws ApiException {
        CollectionType listType = json.getTypeFactory().constructCollectionType(List.class, type);
        try {
            return json.readValue(json.readTree(response.body()).path("data").traverse(json), listType);
        } catch (IOException | IllegalArgumentException e) {
            throw new ApiException(ApiErrorKind.UNEXPECTED, operation, null);
        }
    }

    private <T> T parse(String operation, String body, Class<T> type) throws ApiException {
        try {
            return json.readValue(body, type);
        } catch (JsonProcessingException e) {
            throw new ApiException(ApiErrorKind.UNEXPECTED, operation, null);
        }
    }

    private static String segment(String value) {
        return URLEncoder.encode(value, StandardCharsets.UTF_8);
    }

    private static Duration parseSeconds(String value) {
        try {
            return Duration.ofSeconds(Math.max(0, Long.parseLong(value.strip())));
        } catch (NumberFormatException e) {
            return null;
        }
    }
}
