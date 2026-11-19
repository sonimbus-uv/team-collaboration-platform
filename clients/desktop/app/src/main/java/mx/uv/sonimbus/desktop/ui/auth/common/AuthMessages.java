package mx.uv.sonimbus.desktop.ui.auth.common;

import mx.uv.sonimbus.desktop.domain.registration.RegistrationException;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator.Problem;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;

import java.time.Duration;
import java.util.ArrayList;
import java.util.List;
import java.util.Set;

public final class AuthMessages {
    public static final String EMAIL_TAKEN = "Ya existe una cuenta con este correo.";
    public static final String INVALID_REGISTRATION = "Revisa el correo y la contraseña.";
    public static final String INVALID_EMAIL = "Revisa el correo.";
    public static final String CONNECTION =
            "No se pudo conectar con el servidor. Revisa tu conexión e intenta de nuevo.";
    public static final String EMAIL_DELIVERY =
            "No pudimos enviar el correo de verificación. Intenta de nuevo más tarde.";
    public static final String RATE_LIMITED_NO_WAIT =
            "Demasiadas solicitudes. Intenta de nuevo en unos momentos.";
    public static final String UNEXPECTED = "Ocurrió un error inesperado. Intenta de nuevo.";

    public static final String PASSWORD_RULE =
            "De "
                    + RegistrationValidator.PASSWORD_MIN_LENGTH
                    + " a "
                    + RegistrationValidator.PASSWORD_MAX_LENGTH
                    + " caracteres, con al menos una mayúscula, una minúscula, un número y un"
                    + " símbolo.";

    public static final String LOGIN_FIELDS_EMPTY = "Escribe tu correo y tu contraseña.";
    public static final String INVALID_LOGIN = "Correo o contraseña incorrectos.";

    public static final String VERIFICATION_NOT_SENT =
            "No pudimos enviar el correo de verificación. Usa Reenviar correo.";
    public static final String RESEND_DONE =
            "Enviamos un nuevo código. Usa el más reciente; el plazo original no cambia.";

    public static final String CODE_REQUIRED = "Introduce los 8 dígitos del código.";
    public static final String INVALID_CODE =
            "El código es incorrecto o el intento ya no está disponible.";
    public static final String REGISTRATION_EXPIRED =
            "El intento venció. Inicia un nuevo registro.";
    public static final String REGISTRATION_COMPLETE =
            "Tu cuenta quedó verificada. Ya puedes iniciar sesión.";

    public static String forVerifyEmail(RegistrationException error) {
        return switch (error.kind()) {
            case VALIDATION -> INVALID_CODE;
            case EMAIL_TAKEN -> EMAIL_TAKEN;
            case RATE_LIMITED -> rateLimited(error);
            case NETWORK, TIMEOUT, SERVER, UNAVAILABLE -> CONNECTION;
            case UNEXPECTED -> UNEXPECTED;
        };
    }

    public static final String LOGIN_UNAVAILABLE =
            "El inicio de sesión todavía no está disponible en el servidor.";
    public static final String GOOGLE_UNAVAILABLE =
            "El inicio de sesión con Google todavía no está disponible en el servidor.";
    public static final String RECOVERY_UNAVAILABLE =
            "La recuperación de contraseña todavía no está disponible en el servidor.";

    private AuthMessages() {}

    public static String forRegister(RegistrationException failure) {
        return switch (failure.kind()) {
            case EMAIL_TAKEN -> EMAIL_TAKEN;
            case VALIDATION -> INVALID_REGISTRATION;
            case RATE_LIMITED -> rateLimited(failure);
            case NETWORK, TIMEOUT, SERVER, UNAVAILABLE -> CONNECTION;
            case UNEXPECTED -> UNEXPECTED;
        };
    }

    public static String forResendVerification(RegistrationException failure) {
        return switch (failure.kind()) {
            case VALIDATION -> INVALID_CODE;
            case RATE_LIMITED -> rateLimited(failure);
            case SERVER -> EMAIL_DELIVERY;
            case NETWORK, TIMEOUT, UNAVAILABLE -> CONNECTION;
            case EMAIL_TAKEN, UNEXPECTED -> UNEXPECTED;
        };
    }

    private static String rateLimited(RegistrationException failure) {
        return failure.retryAfter()
                .map(Duration::toSeconds)
                .filter(seconds -> seconds > 0)
                .map(
                        seconds ->
                                "Demasiadas solicitudes. Intenta de nuevo en "
                                        + seconds
                                        + (seconds == 1 ? " segundo." : " segundos."))
                .orElse(RATE_LIMITED_NO_WAIT);
    }

    public static String forLogin(AuthenticationException failure) {
        return switch (failure.kind()) {
            case INVALID_CREDENTIALS -> INVALID_LOGIN;
            case INVALID_INPUT -> "Revisa el correo y la contraseña.";
            case RATE_LIMITED ->
                    failure.retryAfter()
                            .map(Duration::toSeconds)
                            .filter(seconds -> seconds > 0)
                            .map(
                                    seconds ->
                                            "Demasiadas solicitudes. Intenta de nuevo en "
                                                    + seconds
                                                    + " segundos.")
                            .orElse(RATE_LIMITED_NO_WAIT);
            case UNAVAILABLE ->
                    "El servicio de autenticación no está disponible. Intenta más tarde.";
            case NETWORK, TIMEOUT -> CONNECTION;
            case NO_SESSION -> "La sesión terminó. Inicia sesión de nuevo.";
            case UNEXPECTED -> UNEXPECTED;
        };
    }

    public static String forLogout(AuthenticationException failure) {
        return "No se confirmó el cierre de sesión. Puedes reintentar o cancelar. "
                + forLogin(failure);
    }

    public static String forEmail(Set<Problem> problems) {

        if (problems.contains(Problem.EMAIL_EMPTY)) {
            return "Escribe tu correo electrónico.";
        }

        return problems.contains(Problem.EMAIL_INVALID)
                ? "El correo no tiene un formato válido."
                : null;
    }

    public static String forPassword(Set<Problem> problems) {

        if (problems.contains(Problem.PASSWORD_EMPTY)) {
            return "Escribe una contraseña.";
        }

        if (problems.contains(Problem.PASSWORD_TOO_LONG)) {
            return "La contraseña puede tener como máximo "
                    + RegistrationValidator.PASSWORD_MAX_LENGTH
                    + " caracteres.";
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

    public static String forConfirmation(Set<Problem> problems) {

        if (problems.contains(Problem.CONFIRMATION_EMPTY)) {
            return "Confirma tu contraseña.";
        }

        return problems.contains(Problem.CONFIRMATION_MISMATCH)
                ? "Las contraseñas no coinciden."
                : null;
    }

    static String joinSpanish(List<String> items) {

        if (items.size() == 1) {
            return items.get(0);
        }

        return String.join(", ", items.subList(0, items.size() - 1))
                + " y "
                + items.get(items.size() - 1);
    }
}
