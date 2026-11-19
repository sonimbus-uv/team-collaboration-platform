package mx.uv.sonimbus.desktop.domain.password;

public record ChangePasswordRequest(
        String currentPassword, String newPassword, String confirmation) {
    @Override
    public String toString() {
        return "ChangePasswordRequest[REDACTED]";
    }
}
