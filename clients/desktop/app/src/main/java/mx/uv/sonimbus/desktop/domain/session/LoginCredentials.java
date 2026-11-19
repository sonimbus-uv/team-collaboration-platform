package mx.uv.sonimbus.desktop.domain.session;

public record LoginCredentials(String email, String password) {
    @Override
    public String toString() {
        return "LoginCredentials[REDACTED]";
    }
}
