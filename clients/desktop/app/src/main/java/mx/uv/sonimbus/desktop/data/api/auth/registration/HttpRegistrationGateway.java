package mx.uv.sonimbus.desktop.data.api.auth.registration;

import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;
import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.registration.PendingRegistration;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationError;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationException;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationGateway;

import java.util.Objects;
import java.util.Set;
import java.util.UUID;

public final class HttpRegistrationGateway implements RegistrationGateway {
    private final AuthApi api;

    public HttpRegistrationGateway(AuthApi api) {
        this.api = Objects.requireNonNull(api);
    }

    @Override
    public PendingRegistration register(String email, String password)
            throws RegistrationException {
        try {
            return decode(api.register(email, password), "register");
        } catch (ApiException failure) {
            throw translate(failure);
        }
    }

    @Override
    public void verifyEmail(UUID attemptId, String code) throws RegistrationException {
        try {
            Account account = api.verifyEmail(attemptId, code);

            if (account == null || !account.emailVerified()) {
                throw invalidResponse("verify-email");
            }
        } catch (ApiException failure) {
            throw translate(failure);
        }
    }

    @Override
    public PendingRegistration resendVerification(UUID attemptId) throws RegistrationException {
        try {
            PendingRegistration attempt =
                    decode(api.resendVerification(attemptId), "resend-verification");

            if (!attemptId.equals(attempt.attemptId())) {
                throw invalidResponse("resend-verification");
            }

            return attempt;
        } catch (ApiException failure) {
            throw translate(failure);
        }
    }

    private PendingRegistration decode(RegistrationAttempt response, String operation)
            throws RegistrationException {
        try {
            return new PendingRegistration(
                    response.registrationAttemptId(),
                    response.expirationTime(),
                    response.verificationEmailSent());
        } catch (RuntimeException invalidResponse) {
            throw invalidResponse(operation);
        }
    }

    private RegistrationException invalidResponse(String operation) {
        AuthLog.invalidResponse(operation);
        return new RegistrationException(RegistrationError.UNEXPECTED);
    }

    private RegistrationException translate(ApiException failure) {
        RegistrationError kind =
                switch (failure.kind()) {
                    case VALIDATION -> RegistrationError.VALIDATION;
                    case EMAIL_TAKEN -> RegistrationError.EMAIL_TAKEN;
                    case RATE_LIMITED -> RegistrationError.RATE_LIMITED;
                    case NETWORK -> RegistrationError.NETWORK;
                    case TIMEOUT -> RegistrationError.TIMEOUT;
                    case SERVER -> RegistrationError.SERVER;
                    case UNAVAILABLE -> RegistrationError.UNAVAILABLE;
                    case INVALID_CREDENTIALS, UNEXPECTED -> RegistrationError.UNEXPECTED;
                };
        return new RegistrationException(
                kind, failure.retryAfter().orElse(null), Set.of(), failure);
    }
}
