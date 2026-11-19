package mx.uv.sonimbus.desktop.data.api;

import java.time.Duration;
import java.util.Optional;

public class ApiException extends Exception {
    private final ApiErrorKind kind;
    private final int status;
    private final Duration retryAfter;
    private final String problemType;

    public record ResponseDetails(int status, Duration retryAfter, String problemType) {}

    public ApiException(ApiErrorKind kind, String operation, int status, Duration retryAfter) {
        this(new ResponseDetails(status, retryAfter, null), kind, operation);
    }

    public ApiException(ResponseDetails response, ApiErrorKind kind, String operation) {
        super(operation + ": HTTP " + response.status() + " (" + kind + ")");
        this.kind = kind;
        this.status = response.status();
        this.retryAfter = response.retryAfter();
        this.problemType = response.problemType();
    }

    public ApiException(ApiErrorKind kind, String operation, Throwable cause) {
        super(operation + ": " + kind, cause);
        this.kind = kind;
        this.problemType = null;
        this.status = 0;
        this.retryAfter = null;
    }

    public Optional<String> problemType() {
        return Optional.ofNullable(problemType);
    }

    public ApiErrorKind kind() {
        return kind;
    }

    public int status() {
        return status;
    }

    public Optional<Duration> retryAfter() {
        return Optional.ofNullable(retryAfter);
    }
}
