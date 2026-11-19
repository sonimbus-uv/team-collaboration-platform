package mx.uv.sonimbus.desktop.domain.password;

import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator.Problem;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.domain.session.SessionService;

import java.time.Clock;
import java.util.Objects;
import java.util.Set;

public final class PasswordService {
    private final PasswordGateway gateway;
    private final SessionService sessions;
    private final Clock clock;

    public PasswordService(PasswordGateway gateway, SessionService sessions) {
        this(gateway, sessions, Clock.systemUTC());
    }

    public PasswordService(PasswordGateway gateway, SessionService sessions, Clock clock) {
        this.gateway = Objects.requireNonNull(gateway);
        this.sessions = Objects.requireNonNull(sessions);
        this.clock = Objects.requireNonNull(clock);
    }

    public RecoveryAttempt forgotPassword(String email) throws PasswordException {
        String normalized = RegistrationValidator.normalizeEmail(email == null ? "" : email);
        if (!RegistrationValidator.isValidEmailFormat(normalized)) {
            throw new PasswordException(PasswordError.VALIDATION);
        }
        RecoveryAttempt attempt = gateway.forgotPassword(normalized);
        if (!clock.instant().isBefore(attempt.expiresAt())) {
            throw new PasswordException(PasswordError.UNEXPECTED);
        }
        return attempt;
    }

    public void resetPassword(ResetPasswordRequest request) throws PasswordException {
        if (request == null
                || request.recoveryAttemptId() == null
                || request.code() == null
                || !request.code().matches("[0-9]{8}")) {
            throw new PasswordException(PasswordError.INVALID_CODE);
        }
        validateNewPassword(request.newPassword(), request.confirmation());
        try {
            gateway.resetPassword(request);
            sessions.invalidate();
        } catch (PasswordException failure) {
            throw mutationFailure(failure);
        }
    }

    public void changePassword(ChangePasswordRequest request) throws PasswordException {
        if (request == null
                || request.currentPassword() == null
                || request.currentPassword().isEmpty()
                || request.currentPassword().codePointCount(0, request.currentPassword().length())
                        > 128) {
            throw new PasswordException(PasswordError.INVALID_CURRENT);
        }
        validateNewPassword(request.newPassword(), request.confirmation());
        if (request.currentPassword().equals(request.newPassword())) {
            throw new PasswordException(PasswordError.SAME_PASSWORD);
        }
        try {
            sessions.runAuthenticated(token -> changeWithToken(token, request));
        } catch (AuthenticationException failure) {
            if (!sessions.isAuthenticated()) {
                throw new PasswordException(PasswordError.NO_SESSION);
            }
            PasswordError kind =
                    switch (failure.kind()) {
                        case RATE_LIMITED -> PasswordError.RATE_LIMITED;
                        case UNAVAILABLE -> PasswordError.UNAVAILABLE;
                        default -> PasswordError.NO_SESSION;
                    };
            throw new PasswordException(kind, failure.retryAfter().orElse(null), Set.of(), failure);
        }
    }

    private void changeWithToken(String accessToken, ChangePasswordRequest request)
            throws PasswordException {
        try {
            gateway.changePassword(accessToken, request);
            sessions.invalidate();
        } catch (PasswordException failure) {
            if (failure.kind() == PasswordError.NO_SESSION
                    || failure.kind() == PasswordError.CREDENTIALS_CHANGED) {
                sessions.invalidate();
            }
            throw mutationFailure(failure);
        }
    }

    private void validateNewPassword(String password, String confirmation)
            throws PasswordException {
        Set<Problem> problems = RegistrationValidator.validatePassword(password, confirmation);
        if (!problems.isEmpty()) {
            throw new PasswordException(PasswordError.VALIDATION, null, problems, null);
        }
    }

    private PasswordException mutationFailure(PasswordException failure) {
        if (failure.kind() == PasswordError.NETWORK
                || failure.kind() == PasswordError.TIMEOUT
                || failure.kind() == PasswordError.UNCERTAIN
                || failure.kind() == PasswordError.UNEXPECTED) {
            sessions.invalidate();
            return new PasswordException(PasswordError.UNCERTAIN, null, Set.of(), failure);
        }
        return failure;
    }
}
