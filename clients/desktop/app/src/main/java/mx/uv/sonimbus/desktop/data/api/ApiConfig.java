package mx.uv.sonimbus.desktop.data.api;

import java.net.URI;
import java.time.Duration;
import java.util.Locale;
import java.util.Objects;

/**
 * Dirección y tiempos de espera de los servicios.
 *
 * @param authBaseUrl    dirección del Auth Service, sin ruta (las rutas están en {@link AuthPaths})
 * @param coreBaseUrl    dirección del Core Service, sin ruta; el WebSocket usa el mismo host
 * @param connectTimeout espera máxima para abrir la conexión
 * @param requestTimeout espera máxima de la respuesta de cada petición
 */
public record ApiConfig(URI authBaseUrl, URI coreBaseUrl, Duration connectTimeout, Duration requestTimeout) {

    static final String AUTH_URL_PROPERTY = "sonimbus.auth.url";
    static final String AUTH_URL_ENV = "SONIMBUS_AUTH_URL";
    /** Auth en la VM de Vagrant ({@code SERVER_ADDR=0.0.0.0:3001}). */
    static final String DEFAULT_AUTH_URL = "http://192.168.33.30:3001";

    static final String CORE_URL_PROPERTY = "sonimbus.core.url";
    static final String CORE_URL_ENV = "SONIMBUS_CORE_URL";
    /** Core en la VM, por un túnel SSH: Phoenix escucha solo en el loopback de la VM. */
    static final String DEFAULT_CORE_URL = "http://localhost:3002";

    /** RNF-REN-01. */
    public static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(5);
    /** RNF-REN-01. */
    public static final Duration REQUEST_TIMEOUT = Duration.ofSeconds(10);

    public ApiConfig {
        Objects.requireNonNull(authBaseUrl, "authBaseUrl");
        Objects.requireNonNull(coreBaseUrl, "coreBaseUrl");
        Objects.requireNonNull(connectTimeout, "connectTimeout");
        Objects.requireNonNull(requestTimeout, "requestTimeout");
        validateBaseUrl(authBaseUrl);
        validateBaseUrl(coreBaseUrl);
    }

    /** Core en su dirección por omisión. */
    public ApiConfig(URI authBaseUrl, Duration connectTimeout, Duration requestTimeout) {
        this(authBaseUrl, URI.create(DEFAULT_CORE_URL), connectTimeout, requestTimeout);
    }

    /** Configuración con los tiempos de espera de RNF-REN-01. */
    public static ApiConfig withAuthUrl(URI authBaseUrl) {
        return new ApiConfig(authBaseUrl, CONNECT_TIMEOUT, REQUEST_TIMEOUT);
    }

    /** Configuración con los tiempos de espera de RNF-REN-01. */
    public static ApiConfig withUrls(URI authBaseUrl, URI coreBaseUrl) {
        return new ApiConfig(authBaseUrl, coreBaseUrl, CONNECT_TIMEOUT, REQUEST_TIMEOUT);
    }

    /**
     * Toma la dirección de cada servicio de su propiedad ({@code sonimbus.auth.url},
     * {@code sonimbus.core.url}), de su variable de entorno ({@code SONIMBUS_AUTH_URL},
     * {@code SONIMBUS_CORE_URL}) o, si no hay ninguna, de la VM de desarrollo.
     */
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

    /** {@code ws://} o {@code wss://} del WebSocket de Core ({@code /socket/websocket}). */
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
            throw new IllegalArgumentException("La dirección de " + source + " no es una URL válida", e);
        }
    }

    /** Solo http(s) con host y sin credenciales en la URL (RNF-SEG-04). */
    private static void validateBaseUrl(URI url) {
        String scheme = url.getScheme() == null ? "" : url.getScheme().toLowerCase(Locale.ROOT);
        if (!scheme.equals("http") && !scheme.equals("https")) {
            throw new IllegalArgumentException("La dirección del servicio debe usar http o https");
        }
        if (url.getHost() == null) {
            throw new IllegalArgumentException("La dirección del servicio no tiene host");
        }
        if (url.getRawUserInfo() != null) {
            throw new IllegalArgumentException("La dirección del servicio no debe incluir credenciales");
        }
    }
}
