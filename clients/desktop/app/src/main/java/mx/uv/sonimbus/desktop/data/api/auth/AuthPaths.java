package mx.uv.sonimbus.desktop.data.api.auth;

import mx.uv.sonimbus.desktop.data.api.ApiConfig;

/** Rutas del Auth Service, relativas a {@link ApiConfig#authBaseUrl()}. */
final class AuthPaths {

    /** Shared prefix for direct Auth access and gateway access. */
    static final String PREFIX = "/api/v1";

    static final String REGISTER = PREFIX + "/auth/register";
    static final String VERIFY_EMAIL = PREFIX + "/auth/verify-email";
    static final String RESEND_VERIFICATION = PREFIX + "/auth/resend-verification";

    private AuthPaths() {}
}
