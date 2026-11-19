package mx.uv.sonimbus.desktop.data.api.auth;

final class AuthPaths {
    static final String PREFIX = "/api/v1";

    static final String REGISTER = PREFIX + "/auth/register";

    static final String VERIFY_EMAIL = PREFIX + "/auth/verify-email";
    static final String RESEND_VERIFICATION = PREFIX + "/auth/resend-verification";
    static final String GOOGLE_LINK = PREFIX + "/auth/google/link-attempts";
    static final String GOOGLE_REAUTHENTICATE = PREFIX + "/auth/reauthenticate";
    static final String GOOGLE_CONNECTIONS = PREFIX + "/auth/connections";
    static final String GOOGLE_ATTEMPT = PREFIX + "/auth/google/attempts";
    static final String GOOGLE_EXCHANGE = PREFIX + "/auth/google/exchange";
    static final String LOGIN = PREFIX + "/auth/login";
    static final String REFRESH = PREFIX + "/auth/refresh";
    static final String LOGOUT = PREFIX + "/auth/logout";

    static final String CHANGE_PASSWORD = PREFIX + "/auth/change-password";
    static final String FORGOT_PASSWORD = PREFIX + "/auth/forgot-password";
    static final String RESET_PASSWORD = PREFIX + "/auth/reset-password";

    static final String REFRESH_TOKEN = PREFIX + "/auth/refresh";

    private AuthPaths() {}
}
