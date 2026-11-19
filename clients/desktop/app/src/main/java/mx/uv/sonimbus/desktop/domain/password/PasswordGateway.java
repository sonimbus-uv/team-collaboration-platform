package mx.uv.sonimbus.desktop.domain.password;

public interface PasswordGateway {
    RecoveryAttempt forgotPassword(String email) throws PasswordException;

    void resetPassword(ResetPasswordRequest request) throws PasswordException;

    void changePassword(String accessToken, ChangePasswordRequest request) throws PasswordException;
}
