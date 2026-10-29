package mx.uv.sonimbus.desktop.ui.common;

import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.realtime.ChannelConnection.SendException;

/**
 * Textos en español de los errores de Core. Nunca muestran el {@code detail} del servicio ni
 * datos internos.
 */
public final class CoreMessages {

    public static final String CONNECTION =
            "No se pudo conectar con el servidor. Revisa tu conexión e intenta de nuevo.";
    public static final String UNEXPECTED = "Ocurrió un error inesperado. Intenta de nuevo.";
    public static final String SESSION_EXPIRED = "Tu sesión terminó. Inicia sesión de nuevo.";
    public static final String FORBIDDEN = "Tu rol en el grupo no permite esta acción.";
    public static final String NOT_FOUND = "El grupo o el canal ya no existe o no tienes acceso.";
    public static final String INVALID_INVITATION = "La invitación no es válida, caducó o ya no tiene usos.";
    public static final String BANNED = "No puedes unirte a este grupo.";
    public static final String CHANNEL_TAKEN = "Ya existe un canal con ese nombre en el grupo.";
    public static final String VALIDATION = "Revisa los datos e intenta de nuevo.";
    public static final String RATE_LIMITED = "Demasiadas solicitudes. Intenta de nuevo en unos momentos.";
    public static final String MESSAGE_NOT_SENT = "No se pudo enviar el mensaje. Se reintentará al reconectar.";
    public static final String MESSAGE_REJECTED = "El mensaje no es válido: de 1 a 4000 caracteres.";

    private CoreMessages() {
    }

    public static String forError(Throwable error) {
        if (!(error instanceof ApiException e)) {
            return UNEXPECTED;
        }
        return switch (e.kind()) {
            case UNAUTHORIZED -> SESSION_EXPIRED;
            case FORBIDDEN -> FORBIDDEN;
            case NOT_FOUND -> NOT_FOUND;
            case INVALID_INVITATION -> INVALID_INVITATION;
            case BANNED -> BANNED;
            case CONFLICT -> CHANNEL_TAKEN;
            case VALIDATION -> VALIDATION;
            case RATE_LIMITED -> RATE_LIMITED;
            case NETWORK, TIMEOUT, SERVER, UNAVAILABLE -> CONNECTION;
            default -> UNEXPECTED;
        };
    }

    /** Error de {@code message.create} ({@code contracts/events/message.created.md}). */
    public static String forSend(Throwable error) {
        if (error instanceof SendException e) {
            return switch (e.type()) {
                case "validation-error" -> MESSAGE_REJECTED;
                case "not-found" -> NOT_FOUND;
                case "group-archived" -> "El grupo está archivado: solo se puede leer.";
                default -> MESSAGE_NOT_SENT;
            };
        }
        return MESSAGE_NOT_SENT;
    }
}
