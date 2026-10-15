package mx.uv.sonimbus.desktop.data.api;

import java.time.Duration;
import java.util.Optional;

/** Una llamada a un servicio no terminó bien. {@link #error()} indica el caso. */
public class ApiException extends Exception {

    private final ApiError error;
    private final int status;
    private final Duration retryAfter;

    public ApiException(ApiError error, int status, String detail, Duration retryAfter) {
        super(detail);
        this.error = error;
        this.status = status;
        this.retryAfter = retryAfter;
    }

    public ApiException(ApiError error, Throwable cause) {
        super(cause);
        this.error = error;
        this.status = 0;
        this.retryAfter = null;
    }

    public ApiError error() {
        return error;
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
