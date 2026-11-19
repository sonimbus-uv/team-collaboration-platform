package mx.uv.sonimbus.desktop.domain.google;

import mx.uv.sonimbus.desktop.domain.session.SessionTokens;

import java.util.Optional;

public interface GoogleGateway {
    GoogleAttempt createAttempt(String codeChallenge) throws GoogleException;

    SessionTokens exchange(GoogleExchange proof) throws GoogleException;

    GoogleAttempt createLinkAttempt(String accessToken, String codeChallenge)
            throws GoogleException;

    void exchangeLink(GoogleExchange proof) throws GoogleException;

    void reauthenticate(String accessToken, String password) throws GoogleException;

    Optional<GoogleConnection> connection(String accessToken) throws GoogleException;
}
