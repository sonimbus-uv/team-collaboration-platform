package mx.uv.sonimbus.desktop.domain.password;

import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator.Problem;

import java.time.Duration;
import java.util.Optional;
import java.util.Set;

public final class PasswordException extends Exception {
    private final PasswordError kind;
    private final Duration retryAfter;
    private final Set<Problem> problems;

    public PasswordException(PasswordError kind) {
        this(kind, null, Set.of(), null);
    }

    public PasswordException(
            PasswordError kind, Duration retryAfter, Set<Problem> problems, Throwable cause) {
        super("Password operation failed: " + kind, cause);
        this.kind = kind;
        this.retryAfter = retryAfter;
        this.problems = Set.copyOf(problems);
    }

    public PasswordError kind() {
        return kind;
    }

    public Optional<Duration> retryAfter() {
        return Optional.ofNullable(retryAfter);
    }

    public Set<Problem> problems() {
        return problems;
    }
}
