package mx.uv.sonimbus.desktop.domain.registration;

import java.util.Objects;
import java.util.UUID;

public final class RegistrationService {
    private final RegistrationGateway gateway;

    public RegistrationService(RegistrationGateway gateway) {
        this.gateway = Objects.requireNonNull(gateway);
    }

    public RegistrationValidator.Result validate(RegistrationRequest request) {
        if (request == null) {
            return RegistrationValidator.validate(null, null, null);
        }
        return RegistrationValidator.validate(
                request.email(), request.password(), request.confirmation());
    }

    public PendingRegistration register(RegistrationRequest request) throws RegistrationException {
        RegistrationValidator.Result validation = validate(request);

        if (!validation.isValid()) {
            throw new RegistrationException(
                    RegistrationError.VALIDATION, null, validation.problems(), null);
        }

        return gateway.register(validation.email(), request.password());
    }

    public void verifyEmail(UUID attemptId, String code) throws RegistrationException {
        validateAttempt(attemptId);

        if (code == null || !code.matches("[0-9]{8}")) {
            throw new RegistrationException(RegistrationError.VALIDATION);
        }

        gateway.verifyEmail(attemptId, code);
    }

    public PendingRegistration resendVerification(UUID attemptId) throws RegistrationException {
        validateAttempt(attemptId);
        return gateway.resendVerification(attemptId);
    }

    private void validateAttempt(UUID attemptId) throws RegistrationException {
        if (attemptId == null) {
            throw new RegistrationException(RegistrationError.VALIDATION);
        }
    }
}
