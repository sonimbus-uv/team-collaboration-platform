package mx.uv.sonimbus.desktop.data.api;

/** Rutas del Auth Service, relativas a {@link ApiConfig#authBaseUrl()}. */
final class AuthPaths {

    /**
     * Prefijo común. Vacío mientras el escritorio llame directo a Auth; será {@code /api/v1}
     * cuando exista el gateway (#14).
     */
    static final String PREFIX = "";

    static final String REGISTER = PREFIX + "/auth/register";
    static final String VERIFY_EMAIL = PREFIX + "/auth/verify-email";
    static final String RESEND_VERIFICATION = PREFIX + "/auth/resend-verification";

    /** Rutas de sesión: ya incluyen {@code /api/v1} en Auth y en el gateway. */
    static final String LOGIN = "/api/v1/auth/login";
    static final String REFRESH = "/api/v1/auth/refresh";

    private AuthPaths() {
    }
}
