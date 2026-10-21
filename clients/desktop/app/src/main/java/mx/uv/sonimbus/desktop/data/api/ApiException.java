package mx.uv.sonimbus.desktop.data.api;

import java.time.Duration;
import java.util.Optional;

/**
 * Una llamada a un servicio no terminó bien. {@link #kind()} indica el caso.
 *
 * <p>El mensaje solo dice la operación y el código HTTP: nunca incluye el {@code detail} del
 * servicio, ni datos enviados como contraseñas o tokens (RNF-SEG-04).
 */
public class ApiException extends Exception {

    private final ApiErrorKind kind;
    private final int status;
    private final Duration retryAfter;

    public ApiException(ApiErrorKind kind, String operation, int status, Duration retryAfter) {
        super(operation + ": HTTP " + status + " (" + kind + ")");
        this.kind = kind;
        this.status = status;
        this.retryAfter = retryAfter;
    }

    public ApiException(ApiErrorKind kind, String operation, Throwable cause) {
        super(operation + ": " + kind, cause);
        this.kind = kind;
        this.status = 0;
        this.retryAfter = null;
    }

    public ApiErrorKind kind() {
        return kind;
    }

    /** Código HTTP de la respuesta, o 0 si no hubo respuesta. */
    public int status() {
        return status;
    }

    /** Espera indicada por el servicio en {@code Retry-After}, si la envió. */
    public Optional<Duration> retryAfter() {
        return Optional.ofNullable(retryAfter);
    }
}
