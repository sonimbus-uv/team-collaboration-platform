package mx.uv.sonimbus.desktop.domain.password;

public enum PasswordError {
    VALIDATION,
    INVALID_CURRENT,
    INVALID_CODE,
    SAME_PASSWORD,
    NOT_ALLOWED,
    CREDENTIALS_CHANGED,
    NO_SESSION,
    RATE_LIMITED,
    UNAVAILABLE,
    NETWORK,
    TIMEOUT,
    UNCERTAIN,
    UNEXPECTED
}
