package mx.uv.sonimbus.desktop.data.api;

import java.net.URI;

/** Direcciones de los servicios. */
public final class ApiConfig {

    static final String AUTH_URL_PROPERTY = "sonimbus.auth.url";
    static final String AUTH_URL_ENV = "SONIMBUS_AUTH_URL";
    static final String DEFAULT_AUTH_URL = "http://localhost:3001";

    private ApiConfig() {
    }

    /**
     * Dirección del Auth Service: propiedad {@code sonimbus.auth.url}, variable de entorno
     * {@code SONIMBUS_AUTH_URL} o, si no hay ninguna, el servicio local de desarrollo.
     */
    public static URI authBaseUrl() {
        return resolve(System.getProperty(AUTH_URL_PROPERTY), System.getenv(AUTH_URL_ENV));
    }

    static URI resolve(String property, String env) {
        if (property != null && !property.isBlank()) {
            return URI.create(property.strip());
        }
        if (env != null && !env.isBlank()) {
            return URI.create(env.strip());
        }
        return URI.create(DEFAULT_AUTH_URL);
    }
}
