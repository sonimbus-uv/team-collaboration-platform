package mx.uv.sonimbus.desktop.data.api.auth.google;

import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;
import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.google.GoogleAttempt;
import mx.uv.sonimbus.desktop.domain.google.GoogleConnection;
import mx.uv.sonimbus.desktop.domain.google.GoogleError;
import mx.uv.sonimbus.desktop.domain.google.GoogleException;
import mx.uv.sonimbus.desktop.domain.google.GoogleExchange;
import mx.uv.sonimbus.desktop.domain.google.GoogleGateway;
import mx.uv.sonimbus.desktop.domain.session.SessionTokens;

import java.time.Instant;
import java.util.Objects;
import java.util.Optional;

public final class HttpGoogleGateway implements GoogleGateway {
    private final AuthApi api;

    public HttpGoogleGateway(AuthApi api) {
        this.api = Objects.requireNonNull(api);
    }

    @Override
    public GoogleAttempt createAttempt(String challenge) throws GoogleException {
        try {
            return api.createGoogleAttempt(challenge).toAttempt();
        } catch (ApiException failure) {
            throw translate(failure, false);
        } catch (RuntimeException malformed) {
            AuthLog.invalidResponse("google-attempt");
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        }
    }

    @Override
    public SessionTokens exchange(GoogleExchange proof) throws GoogleException {
        Instant started = Instant.now();
        try {
            return api.exchangeGoogle(proof).toSessionTokens(started);
        } catch (ApiException failure) {
            throw translate(failure, true);
        } catch (RuntimeException malformed) {
            AuthLog.invalidResponse("google-exchange");
            throw new GoogleException(GoogleError.UNCERTAIN);
        }
    }

    @Override
    public GoogleAttempt createLinkAttempt(String accessToken, String challenge)
            throws GoogleException {
        try {
            return api.createGoogleLinkAttempt(accessToken, challenge).toAttempt();
        } catch (ApiException failure) {
            throw translate(failure, false);
        } catch (RuntimeException malformed) {
            AuthLog.invalidResponse("google-link");
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        }
    }

    @Override
    public void exchangeLink(GoogleExchange proof) throws GoogleException {
        try {
            api.exchangeGoogleLink(proof);
        } catch (ApiException failure) {
            throw translate(failure, true);
        }
    }

    @Override
    public void reauthenticate(String accessToken, String password) throws GoogleException {
        try {
            api.reauthenticateGoogle(accessToken, password);
        } catch (ApiException failure) {
            throw translate(failure, false);
        }
    }

    @Override
    public Optional<GoogleConnection> connection(String accessToken) throws GoogleException {
        try {
            return api.googleConnections(accessToken).toConnection();
        } catch (ApiException failure) {
            throw translate(failure, false);
        } catch (RuntimeException malformed) {
            AuthLog.invalidResponse("google-connections");
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        }
    }

    private GoogleException translate(ApiException failure, boolean exchanging) {
        GoogleError kind =
                switch (failure.status()) {
                    case 401 -> unauthorized(failure);
                    case 403 ->
                            failure.problemType()
                                            .filter("/problems/reauthentication-required"::equals)
                                            .isPresent()
                                    ? GoogleError.REAUTHENTICATION_REQUIRED
                                    : GoogleError.ACCOUNT_BLOCKED;
                    case 409 -> conflict(failure);
                    case 429 -> GoogleError.RATE_LIMITED;
                    case 503 -> GoogleError.UNAVAILABLE;
                    default -> exchanging ? GoogleError.UNCERTAIN : GoogleError.NETWORK;
                };
        return new GoogleException(kind, failure.retryAfter().orElse(null), failure);
    }

    private GoogleError unauthorized(ApiException failure) {
        return switch (failure.problemType().orElse("")) {
            case "/problems/reauthentication-required" -> GoogleError.REAUTHENTICATION_REQUIRED;
            case "/problems/invalid-access-token" -> GoogleError.NO_SESSION;
            case "/problems/invalid-credentials" -> GoogleError.INVALID_PASSWORD;
            default -> GoogleError.INVALID_RESPONSE;
        };
    }

    private GoogleError conflict(ApiException failure) {
        return switch (failure.problemType().orElse("")) {
            case "/problems/account-link-required" -> GoogleError.LINK_REQUIRED;
            case "/problems/provider-already-linked" -> GoogleError.ALREADY_LINKED;
            case "/problems/identity-already-linked", "/problems/account-link-conflict" ->
                    GoogleError.LINK_CONFLICT;
            default -> GoogleError.INVALID_RESPONSE;
        };
    }
}
