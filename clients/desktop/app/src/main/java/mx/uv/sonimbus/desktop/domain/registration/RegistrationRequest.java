package mx.uv.sonimbus.desktop.domain.registration;

public record RegistrationRequest(String email, String password, String confirmation) {
    @Override
    public String toString() {
        return "RegistrationRequest[REDACTED]";
    }
}
