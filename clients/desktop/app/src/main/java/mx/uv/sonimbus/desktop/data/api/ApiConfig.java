package mx.uv.sonimbus.desktop.data.api;

import java.net.URI;
import java.time.Duration;
import java.util.Locale;
import java.util.Objects;

/**
 * Dirección y tiempos de espera de los servicios.
 *
 * @param authBaseUrl    dirección del Auth Service, sin ruta (las rutas están en {@link AuthPaths})
 * @param connectTimeout espera máxima para abrir la conexión
 * @param requestTimeout espera máxima de la respuesta de cada petición
 */
public record ApiConfig(URI authBaseUrl, Duration connectTimeout, Duration requestTimeout) {

    static final String AUTH_URL_PROPERTY = "sonimbus.auth.url";
    static final String AUTH_URL_ENV = "SONIMBUS_AUTH_URL";
    /** Auth en la VM de Vagrant ({@code SERVER_ADDR=0.0.0.0:3001}). */
    static final String DEFAULT_AUTH_URL = "http://192.168.33.30:3001";

    /** RNF-REN-01. */
    public static final Duration CONNECT_TIMEOUT = Duration.ofSeconds(5);
    /** RNF-REN-01. */
    public static final Duration REQUEST_TIMEOUT = Duration.ofSeconds(10);

    public ApiConfig {
        Objects.requireNonNull(authBaseUrl, "authBaseUrl");
        Objects.requireNonNull(connectTimeout, "connectTimeout");
        Objects.requireNonNull(requestTimeout, "requestTimeout");
        validateBaseUrl(authBaseUrl);
    }

    /** Configuración con los tiempos de espera de RNF-REN-01. */
    public static ApiConfig withAuthUrl(URI authBaseUrl) {
        return new ApiConfig(authBaseUrl, CONNECT_TIMEOUT, REQUEST_TIMEOUT);
    }

    /**
     * Toma la dirección del Auth Service de la propiedad {@code sonimbus.auth.url}, de la
     * variable de entorno {@code SONIMBUS_AUTH_URL} o, si no hay ninguna, de la VM de desarrollo.
     */
    public static ApiConfig fromEnvironment() {
        return withAuthUrl(resolveAuthUrl(System.getProperty(AUTH_URL_PROPERTY), System.getenv(AUTH_URL_ENV)));
    }

    static URI resolveAuthUrl(String property, String env) {
        if (property != null && !property.isBlank()) {
            return parse(property, AUTH_URL_PROPERTY);
        }
        if (env != null && !env.isBlank()) {
            return parse(env, AUTH_URL_ENV);
        }
        return URI.create(DEFAULT_AUTH_URL);
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
