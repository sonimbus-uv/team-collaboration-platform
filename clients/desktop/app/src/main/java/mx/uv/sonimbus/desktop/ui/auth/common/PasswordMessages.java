package mx.uv.sonimbus.desktop.ui.auth.common;

import mx.uv.sonimbus.desktop.domain.password.PasswordException;

import java.time.Duration;
import java.util.ArrayList;
import java.util.List;

public final class PasswordMessages {
    public static final String RECOVERY_ACCEPTED =
            "Si la cuenta permite recuperar la contraseña, recibirás un código por correo. "
                    + "Usa el código de tu última solicitud en esta ventana.";
    public static final String UPDATED = "Tu contraseña se actualizó. Inicia sesión de nuevo.";
    public static final String UNCERTAIN =
            "No se confirmó el cambio. Intenta iniciar sesión con la nueva contraseña. "
                    + "Si no funciona, solicita una nueva recuperación.";

    private PasswordMessages() {}

    public static String failure(PasswordException failure) {
        return switch (failure.kind()) {
            case VALIDATION -> validation(failure);
            case INVALID_CURRENT -> "La contraseña actual es incorrecta o no es válida.";
            case INVALID_CODE -> "El código es incorrecto o el intento ya no está disponible.";
            case SAME_PASSWORD -> "La nueva contraseña debe ser diferente de la actual.";
            case NOT_ALLOWED -> "Esta cuenta no permite cambiar una contraseña local.";
            case CREDENTIALS_CHANGED ->
                    "La contraseña cambió durante la operación. Inicia sesión de nuevo.";
            case NO_SESSION -> "La sesión terminó. Inicia sesión de nuevo.";
            case RATE_LIMITED ->
                    failure.retryAfter()
                            .map(Duration::toSeconds)
                            .filter(seconds -> seconds > 0)
                            .map(
                                    seconds ->
                                            "Demasiadas solicitudes. Intenta de nuevo en "
                                                    + seconds
                                                    + " segundos.")
                            .orElse(AuthMessages.RATE_LIMITED_NO_WAIT);
            case UNAVAILABLE -> "El servicio de contraseñas no está disponible. Intenta más tarde.";
            case NETWORK, TIMEOUT -> AuthMessages.CONNECTION;
            case UNCERTAIN -> UNCERTAIN;
            case UNEXPECTED -> AuthMessages.UNEXPECTED;
        };
    }

    public static String recoveryFailure(PasswordException failure) {
        return switch (failure.kind()) {
            case UNCERTAIN, NETWORK, TIMEOUT ->
                    "No se confirmó la solicitud del código. Espera un minuto y solicita otro.";
            case VALIDATION -> "Escribe un correo electrónico válido.";
            default -> failure(failure);
        };
    }

    private static String validation(PasswordException failure) {
        List<String> messages = new ArrayList<>();
        String password = AuthMessages.forPassword(failure.problems());
        String confirmation = AuthMessages.forConfirmation(failure.problems());
        if (password != null) {
            messages.add(password);
        }
        if (confirmation != null) {
            messages.add(confirmation);
        }
        return messages.isEmpty()
                ? "Revisa los datos y las reglas de la nueva contraseña."
                : String.join("\n", messages);
    }
}
