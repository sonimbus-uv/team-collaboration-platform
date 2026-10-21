package mx.uv.sonimbus.desktop.ui.auth;

import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.domain.session.RegistrationValidator;
import mx.uv.sonimbus.desktop.domain.session.RegistrationValidator.Problem;

/**
 * Textos en español de las pantallas de autenticación: errores de Auth, validación del
 * formulario y avisos de lo que el servidor todavía no ofrece. Los errores nunca muestran el
 * {@code detail} del servicio ni datos internos.
 */
public final class AuthMessages {

    static final String EMAIL_TAKEN = "Ya existe una cuenta con este correo.";
    static final String INVALID_REGISTRATION = "Revisa el correo y la contraseña.";
    static final String INVALID_EMAIL = "Revisa el correo.";
    static final String CONNECTION = "No se pudo conectar con el servidor. Revisa tu conexión e intenta de nuevo.";
    static final String EMAIL_DELIVERY = "No pudimos enviar el correo de verificación. Intenta de nuevo más tarde.";
    static final String RATE_LIMITED_NO_WAIT = "Demasiadas solicitudes. Intenta de nuevo en unos momentos.";
    static final String UNEXPECTED = "Ocurrió un error inesperado. Intenta de nuevo.";

    /** Regla real de Auth ({@code auth_validator.rs}); se repite en el FXML y su accessibleHelp. */
    public static final String PASSWORD_RULE = "De " + RegistrationValidator.PASSWORD_MIN_LENGTH + " a "
            + RegistrationValidator.PASSWORD_MAX_LENGTH
            + " caracteres, con al menos una mayúscula, una minúscula, un número y un símbolo.";

    public static final String LOGIN_FIELDS_EMPTY = "Escribe tu correo y tu contraseña.";
    public static final String LOGIN_UNAVAILABLE = "El inicio de sesión todavía no está disponible en el servidor.";
    public static final String GOOGLE_UNAVAILABLE =
            "El inicio de sesión con Google todavía no está disponible en el servidor.";
    public static final String RECOVERY_UNAVAILABLE =
            "La recuperación de contraseña todavía no está disponible en el servidor.";

    public static final String VERIFICATION_NOT_SENT =
            "No pudimos enviar el correo de verificación. Usa Reenviar correo.";
    public static final String RESEND_DONE = "Si la cuenta existe y no está verificada, te enviamos un nuevo enlace.";

    private AuthMessages() {
    }

    /** CU-01. */
    public static String forRegister(ApiException e) {
        return switch (e.kind()) {
            case EMAIL_TAKEN -> EMAIL_TAKEN;
            case VALIDATION -> INVALID_REGISTRATION;
            case RATE_LIMITED -> rateLimited(e);
            case NETWORK, TIMEOUT, SERVER, UNAVAILABLE -> CONNECTION;
            case UNEXPECTED -> UNEXPECTED;
        };
    }

    /** CU-02. En el reenvío, un 500 casi siempre significa que el correo no se pudo enviar. */
    public static String forResendVerification(ApiException e) {
        return switch (e.kind()) {
            case VALIDATION -> INVALID_EMAIL;
            case RATE_LIMITED -> rateLimited(e);
            case SERVER -> EMAIL_DELIVERY;
            case NETWORK, TIMEOUT, UNAVAILABLE -> CONNECTION;
            case EMAIL_TAKEN, UNEXPECTED -> UNEXPECTED;
        };
    }

    private static String rateLimited(ApiException e) {
        return e.retryAfter()
                .map(Duration::toSeconds)
                .filter(seconds -> seconds > 0)
                .map(seconds -> "Demasiadas solicitudes. Intenta de nuevo en " + seconds
                        + (seconds == 1 ? " segundo." : " segundos."))
                .orElse(RATE_LIMITED_NO_WAIT);
    }

    /** Mensaje del campo de correo, o {@code null} si no tiene problemas. */
    public static String forEmail(Set<Problem> problems) {
        if (problems.contains(Problem.EMAIL_EMPTY)) {
            return "Escribe tu correo electrónico.";
        }
        return problems.contains(Problem.EMAIL_INVALID) ? "El correo no tiene un formato válido." : null;
    }

    /**
     * Mensaje del campo de contraseña con todo lo que le falta, por ejemplo «La contraseña
     * necesita al menos 8 caracteres, una mayúscula y un símbolo.»; {@code null} si es válida.
     */
    public static String forPassword(Set<Problem> problems) {
        if (problems.contains(Problem.PASSWORD_EMPTY)) {
            return "Escribe una contraseña.";
        }
        if (problems.contains(Problem.PASSWORD_TOO_LONG)) {
            return "La contraseña puede tener como máximo " + RegistrationValidator.PASSWORD_MAX_LENGTH + " caracteres.";
        }
        List<String> missing = new ArrayList<>();
        if (problems.contains(Problem.PASSWORD_TOO_SHORT)) {
            missing.add("al menos " + RegistrationValidator.PASSWORD_MIN_LENGTH + " caracteres");
        }
        if (problems.contains(Problem.PASSWORD_MISSING_UPPERCASE)) {
            missing.add("una mayúscula");
        }
        if (problems.contains(Problem.PASSWORD_MISSING_LOWERCASE)) {
            missing.add("una minúscula");
        }
        if (problems.contains(Problem.PASSWORD_MISSING_DIGIT)) {
            missing.add("un número");
        }
        if (problems.contains(Problem.PASSWORD_MISSING_SYMBOL)) {
            missing.add("un símbolo (por ejemplo ! o #)");
        }
        return missing.isEmpty() ? null : "La contraseña necesita " + joinSpanish(missing) + ".";
    }

    /** Mensaje del campo de confirmación, o {@code null} si coincide. */
    public static String forConfirmation(Set<Problem> problems) {
        if (problems.contains(Problem.CONFIRMATION_EMPTY)) {
            return "Confirma tu contraseña.";
        }
        return problems.contains(Problem.CONFIRMATION_MISMATCH) ? "Las contraseñas no coinciden." : null;
    }

    /** «a», «a y b», «a, b y c». */
    static String joinSpanish(List<String> items) {
        if (items.size() == 1) {
            return items.get(0);
        }
        return String.join(", ", items.subList(0, items.size() - 1)) + " y " + items.get(items.size() - 1);
    }
}
