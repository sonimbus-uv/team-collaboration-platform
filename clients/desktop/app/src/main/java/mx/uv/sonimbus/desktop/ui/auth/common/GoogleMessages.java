package mx.uv.sonimbus.desktop.ui.auth.common;

import mx.uv.sonimbus.desktop.domain.google.GoogleException;

import java.time.Duration;

public final class GoogleMessages {
    private GoogleMessages() {}

    public static String failure(GoogleException failure) {
        return switch (failure.kind()) {
            case REAUTHENTICATION_REQUIRED -> "Confirma tu contraseña para vincular Google.";
            case INVALID_PASSWORD -> "La contraseña es incorrecta o no es válida para esta cuenta.";
            case ALREADY_LINKED -> "Esta cuenta ya tiene una identidad Google vinculada.";
            case LINK_CONFLICT ->
                    "Esa identidad Google o su correo pertenece a otra cuenta. No se realizó la"
                        + " vinculación.";
            case NO_SESSION -> "La sesión terminó. Inicia sesión de nuevo.";
            case CANCELLED -> "Se canceló el inicio de sesión con Google.";
            case EXPIRED -> "El intento de Google venció. Inicia uno nuevo.";
            case INVALID_RESPONSE ->
                    "No se pudo validar el inicio de sesión con Google. Intenta de nuevo.";
            case LINK_REQUIRED ->
                    "Ya existe una cuenta con este correo. Accede con su método actual;"
                            + " no se vincularon las cuentas.";
            case ACCOUNT_BLOCKED -> "Esta cuenta no tiene permitido iniciar sesión.";
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
            case UNAVAILABLE -> "El acceso con Google no está disponible. Intenta más tarde.";
            case NETWORK -> AuthMessages.CONNECTION;
            case UNCERTAIN ->
                    "No se confirmó el inicio de sesión. Inicia un nuevo intento con Google.";
            case BROWSER -> "No se pudo abrir el navegador o recibir el retorno de Google.";
            case PLATFORM -> "No se pudo registrar el retorno automático de Google para Windows.";
            case BUSY -> "Ya hay un inicio de sesión con Google en curso.";
        };
    }
}
