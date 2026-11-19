package mx.uv.sonimbus.desktop.data.api.auth.session;

import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;
import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationError;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationGateway;
import mx.uv.sonimbus.desktop.domain.session.LoginCredentials;
import mx.uv.sonimbus.desktop.domain.session.SessionTokens;

import java.time.Clock;
import java.time.Instant;
import java.util.Objects;

public final class HttpAuthenticationGateway implements AuthenticationGateway {
    private final AuthApi api;
    private final Clock clock;

    public HttpAuthenticationGateway(AuthApi api) {
        this(api, Clock.systemUTC());
    }

    public HttpAuthenticationGateway(AuthApi api, Clock clock) {
        this.api = Objects.requireNonNull(api);
        this.clock = Objects.requireNonNull(clock);
    }

    @Override
    public SessionTokens login(LoginCredentials credentials) throws AuthenticationException {
        Instant started = clock.instant();
        try {
            return decode(api.login(credentials.email(), credentials.password()), started, "login");
        } catch (ApiException failure) {
            throw translate(failure, false);
        }
    }

    @Override
    public SessionTokens refresh(String refreshToken) throws AuthenticationException {
        Instant started = clock.instant();
        try {
            return decode(api.refresh(refreshToken), started, "refresh");
        } catch (ApiException failure) {
            throw translate(failure, true);
        }
    }

    @Override
    public void logout(String refreshToken) throws AuthenticationException {
        try {
            api.logout(refreshToken);
        } catch (ApiException failure) {
            throw translate(failure, false);
        }
    }

    private SessionTokens decode(TokenResponse response, Instant started, String operation)
            throws AuthenticationException {
        try {
            return response.toSessionTokens(started);
        } catch (RuntimeException invalidResponse) {
            AuthLog.invalidResponse(operation);
            throw new AuthenticationException(AuthenticationError.UNEXPECTED);
        }
    }

    private AuthenticationException translate(ApiException failure, boolean refreshing) {
        if (refreshing && failure.status() == 401) {
            return new AuthenticationException(AuthenticationError.NO_SESSION, null, failure);
        }
        AuthenticationError kind =
                switch (failure.kind()) {
                    case VALIDATION -> AuthenticationError.INVALID_INPUT;
                    case INVALID_CREDENTIALS -> AuthenticationError.INVALID_CREDENTIALS;
                    case RATE_LIMITED -> AuthenticationError.RATE_LIMITED;
                    case NETWORK -> AuthenticationError.NETWORK;
                    case TIMEOUT -> AuthenticationError.TIMEOUT;
                    case UNAVAILABLE ->
                            refreshing && failure.status() != 503
                                    ? AuthenticationError.NETWORK
                                    : AuthenticationError.UNAVAILABLE;
                    default -> AuthenticationError.UNEXPECTED;
                };
        return new AuthenticationException(kind, failure.retryAfter().orElse(null), failure);
    }
}
