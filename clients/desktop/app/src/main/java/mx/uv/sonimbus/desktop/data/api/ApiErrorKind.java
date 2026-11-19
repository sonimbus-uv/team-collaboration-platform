package mx.uv.sonimbus.desktop.data.api;

public enum ApiErrorKind {
    VALIDATION,
    EMAIL_TAKEN,
    INVALID_CREDENTIALS,
    RATE_LIMITED,
    SERVER,
    UNAVAILABLE,
    NETWORK,
    TIMEOUT,
    UNEXPECTED
}
