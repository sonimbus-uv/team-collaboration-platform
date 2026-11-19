package mx.uv.sonimbus.desktop.data.api.auth.registration;

/** Estado de la cuenta que devuelven el registro y la verificación de correo. */
public record Account(
        String userId,
        String email,
        boolean emailVerified,
        String status,
        boolean verificationEmailSent) {}
