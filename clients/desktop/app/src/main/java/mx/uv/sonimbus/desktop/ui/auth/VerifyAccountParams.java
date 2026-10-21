package mx.uv.sonimbus.desktop.ui.auth;

import java.util.Objects;

/**
 * Datos que recibe {@code VERIFY_ACCOUNT}.
 *
 * @param email    correo al que se envió (o debía enviarse) el enlace
 * @param delivery qué se sabe del correo de verificación
 */
public record VerifyAccountParams(String email, Delivery delivery) {

    public enum Delivery {
        /** Auth lo acaba de enviar al registrar: su cooldown de reenvío ya corre. */
        SENT,
        /** Auth registró la cuenta pero no pudo enviarlo ({@code verification_email_sent=false}). */
        FAILED,
        /** Se llega desde un 409 («ya existe una cuenta»): no se sabe si se envió ni cuándo. */
        UNKNOWN
    }

    public VerifyAccountParams {
        Objects.requireNonNull(email, "email");
        Objects.requireNonNull(delivery, "delivery");
    }

    /** Respuesta 201 del registro. */
    public static VerifyAccountParams afterRegistration(String email, boolean verificationEmailSent) {
        return new VerifyAccountParams(email, verificationEmailSent ? Delivery.SENT : Delivery.FAILED);
    }

    /** Enlace «¿No recibiste el correo de verificación?» del 409. */
    public static VerifyAccountParams forExistingAccount(String email) {
        return new VerifyAccountParams(email, Delivery.UNKNOWN);
    }
}
