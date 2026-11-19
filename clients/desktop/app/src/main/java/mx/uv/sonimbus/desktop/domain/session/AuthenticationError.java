package mx.uv.sonimbus.desktop.domain.session;

public enum AuthenticationError {
    INVALID_INPUT,
    INVALID_CREDENTIALS,
    RATE_LIMITED,
    UNAVAILABLE,
    NETWORK,
    TIMEOUT,
    NO_SESSION,
    UNEXPECTED
}
