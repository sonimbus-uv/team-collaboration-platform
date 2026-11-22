package mx.uv.sonimbus.desktop.data.api;

public enum ApiErrorKind {
    VALIDATION,
    EMAIL_TAKEN,
    INVALID_CREDENTIALS,
    UNAUTHORIZED,
    FORBIDDEN,
    BANNED,
    NOT_FOUND,
    CONFLICT,
    INVALID_INVITATION,
    RATE_LIMITED,
    SERVER,
    UNAVAILABLE,
    NETWORK,
    TIMEOUT,
    UNEXPECTED
}
