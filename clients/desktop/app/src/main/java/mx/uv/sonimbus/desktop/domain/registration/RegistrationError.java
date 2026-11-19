package mx.uv.sonimbus.desktop.domain.registration;

public enum RegistrationError {
    VALIDATION,
    EMAIL_TAKEN,
    RATE_LIMITED,
    NETWORK,
    TIMEOUT,
    SERVER,
    UNAVAILABLE,
    UNEXPECTED
}
