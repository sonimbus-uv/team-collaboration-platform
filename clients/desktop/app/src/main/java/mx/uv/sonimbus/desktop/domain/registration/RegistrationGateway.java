package mx.uv.sonimbus.desktop.domain.registration;

import java.util.UUID;

public interface RegistrationGateway {
    PendingRegistration register(String email, String password) throws RegistrationException;

    void verifyEmail(UUID attemptId, String code) throws RegistrationException;

    PendingRegistration resendVerification(UUID attemptId) throws RegistrationException;
}
