package mx.uv.sonimbus.desktop.data.api.auth.password;

import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;
import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.password.ChangePasswordRequest;
import mx.uv.sonimbus.desktop.domain.password.PasswordError;
import mx.uv.sonimbus.desktop.domain.password.PasswordException;
import mx.uv.sonimbus.desktop.domain.password.PasswordGateway;
import mx.uv.sonimbus.desktop.domain.password.RecoveryAttempt;
import mx.uv.sonimbus.desktop.domain.password.ResetPasswordRequest;

import java.util.Objects;
import java.util.Set;

public final class HttpPasswordGateway implements PasswordGateway {
    private final AuthApi api;

    public HttpPasswordGateway(AuthApi api) {
        this.api = Objects.requireNonNull(api);
    }

    @Override
    public RecoveryAttempt forgotPassword(String email) throws PasswordException {
        try {
            RecoveryResponse response = api.forgotPassword(email);
            try {
                return response.toAttempt();
            } catch (RuntimeException invalidResponse) {
                AuthLog.invalidResponse("forgot-password");
                throw new PasswordException(PasswordError.UNEXPECTED);
            }
        } catch (ApiException failure) {
            throw translate(failure);
        }
    }

    @Override
    public void resetPassword(ResetPasswordRequest request) throws PasswordException {
        try {
            api.resetPassword(request.recoveryAttemptId(), request.code(), request.newPassword());
        } catch (ApiException failure) {
            throw translate(failure);
        }
    }

    @Override
    public void changePassword(String accessToken, ChangePasswordRequest request)
            throws PasswordException {
        try {
            api.changePassword(accessToken, request.currentPassword(), request.newPassword());
        } catch (ApiException failure) {
            throw translate(failure);
        }
    }

    private PasswordException translate(ApiException failure) {
        PasswordError kind =
                switch (failure.status()) {
                    case 400 -> badRequest(failure);
                    case 401 -> PasswordError.NO_SESSION;
                    case 409 -> conflict(failure);
                    case 500, 502, 504 -> PasswordError.UNCERTAIN;
                    default ->
                            switch (failure.kind()) {
                                case RATE_LIMITED -> PasswordError.RATE_LIMITED;
                                case UNAVAILABLE -> PasswordError.UNAVAILABLE;
                                case NETWORK -> PasswordError.NETWORK;
                                case TIMEOUT -> PasswordError.TIMEOUT;
                                default -> PasswordError.UNEXPECTED;
                            };
                };
        return new PasswordException(kind, failure.retryAfter().orElse(null), Set.of(), failure);
    }

    private PasswordError badRequest(ApiException failure) {
        return switch (failure.problemType().orElse("")) {
            case "/problems/invalid-current-password" -> PasswordError.INVALID_CURRENT;
            case "/problems/invalid-recovery-code" -> PasswordError.INVALID_CODE;
            case "/problems/invalid-password", "/problems/malformed-request" ->
                    PasswordError.VALIDATION;
            default -> PasswordError.UNEXPECTED;
        };
    }

    private PasswordError conflict(ApiException failure) {
        return switch (failure.problemType().orElse("")) {
            case "/problems/password-unchanged" -> PasswordError.SAME_PASSWORD;
            case "/problems/password-change-not-allowed" -> PasswordError.NOT_ALLOWED;
            case "/problems/credentials-changed" -> PasswordError.CREDENTIALS_CHANGED;
            default -> PasswordError.UNEXPECTED;
        };
    }
}
