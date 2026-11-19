package mx.uv.sonimbus.desktop.data.api.auth.registration;

public record Account(String userId, String email, boolean emailVerified, String status) {}
