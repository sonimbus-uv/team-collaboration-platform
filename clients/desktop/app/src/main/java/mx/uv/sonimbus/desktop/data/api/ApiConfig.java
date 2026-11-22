package mx.uv.sonimbus.desktop.data.api;

import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;

import java.net.URI;
import java.time.Duration;
import java.util.Locale;
import java.util.Objects;

public record ApiConfig(URI authBaseUrl, URI coreBaseUrl, Duration connectTimeout, Duration requestTimeout) {

    static final String AUTH_URL_PROPERTY = "sonimbus.auth.url";
    static final String AUTH_URL_ENV = "SONIMBUS_AUTH_URL";

    static final String DEFAULT_AUTH_URL = "http://192.168.33.30:3001";

    static final String CORE_URL_PROPERTY = "sonimbus.core.url";
    static final String CORE_URL_ENV = "SONIMBUS_CORE_URL";
    static final String DEFAULT_CORE_URL = "http://localhost:3002";

    public static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(5);

    public static final Duration REQUEST_TIMEOUT = Duration.ofSeconds(10);

    public ApiConfig {
        Objects.requireNonNull(authBaseUrl, "authBaseUrl");
        Objects.requireNonNull(coreBaseUrl, "coreBaseUrl");
        Objects.requireNonNull(connectTimeout, "connectTimeout");
        Objects.requireNonNull(requestTimeout, "requestTimeout");
        validateBaseUrl(authBaseUrl);
        validateBaseUrl(coreBaseUrl);
    }

    public ApiConfig(URI authBaseUrl, Duration connectTimeout, Duration requestTimeout) {
        this(authBaseUrl, URI.create(DEFAULT_CORE_URL), connectTimeout, requestTimeout);
    }

    public static ApiConfig withAuthUrl(URI authBaseUrl) {
        return new ApiConfig(authBaseUrl, CONNECT_TIMEOUT, REQUEST_TIMEOUT);
    }

    public static ApiConfig withUrls(URI authBaseUrl, URI coreBaseUrl) {
        return new ApiConfig(authBaseUrl, coreBaseUrl, CONNECT_TIMEOUT, REQUEST_TIMEOUT);
    }

    public static ApiConfig fromEnvironment() {
        return withUrls(
                resolveAuthUrl(System.getProperty(AUTH_URL_PROPERTY), System.getenv(AUTH_URL_ENV)),
                resolveCoreUrl(System.getProperty(CORE_URL_PROPERTY), System.getenv(CORE_URL_ENV)));
    }

    static URI resolveAuthUrl(String property, String env) {
        return resolve(property, AUTH_URL_PROPERTY, env, AUTH_URL_ENV, DEFAULT_AUTH_URL);
    }

    static URI resolveCoreUrl(String property, String env) {
        return resolve(property, CORE_URL_PROPERTY, env, CORE_URL_ENV, DEFAULT_CORE_URL);
    }

    public URI coreSocketUrl() {
        String scheme = coreBaseUrl.getScheme().equalsIgnoreCase("https") ? "wss" : "ws";
        String base = coreBaseUrl.toString().replaceAll("/+$", "");
        return URI.create(scheme + base.substring(base.indexOf(':')) + "/socket/websocket?vsn=2.0.0");
    }

    private static URI resolve(String property, String propertyName, String env, String envName, String fallback) {
        if (property != null && !property.isBlank()) {
            return parse(property, propertyName);
        }
        if (env != null && !env.isBlank()) {
            return parse(env, envName);
        }
        return URI.create(fallback);
    }

    private static URI parse(String value, String source) {
        try {
            return URI.create(value.strip().replaceAll("/+$", ""));
        } catch (IllegalArgumentException e) {
            throw new IllegalArgumentException(
                    "La dirección de " + source + " no es una URL válida", e);
        }
    }

    private static void validateBaseUrl(URI url) {
        String scheme = url.getScheme() == null ? "" : url.getScheme().toLowerCase(Locale.ROOT);
        if (!scheme.equals("http") && !scheme.equals("https")) {
            throw new IllegalArgumentException("La dirección del servicio debe usar http o https");
        }
        if (url.getHost() == null) {
            throw new IllegalArgumentException("La dirección del servicio no tiene host");
        }
        if (url.getRawUserInfo() != null) {
            throw new IllegalArgumentException(
                    "La dirección del servicio no debe incluir credenciales");
        }
    }
}
