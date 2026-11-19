package mx.uv.sonimbus.desktop.domain.session;

public interface AuthenticationGateway {
    SessionTokens login(LoginCredentials credentials) throws AuthenticationException;

    SessionTokens refresh(String refreshToken) throws AuthenticationException;

    void logout(String refreshToken) throws AuthenticationException;
}
