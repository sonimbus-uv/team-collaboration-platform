package mx.uv.sonimbus.desktop.domain.session;

import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator;

import java.time.Clock;
import java.util.Objects;
import java.util.Optional;

public final class SessionService implements AutoCloseable {
    private final AuthenticationGateway gateway;
    private final Clock clock;
    private final Object operationLock = new Object();
    private final Object stateLock = new Object();
    private volatile SignedInSession session;
    private boolean closed;

    private record SignedInSession(String email, SessionTokens tokens) {
        @Override
        public String toString() {
            return "SignedInSession[REDACTED]";
        }
    }

    public SessionService(AuthenticationGateway gateway) {
        this(gateway, Clock.systemUTC());
    }

    public SessionService(AuthenticationGateway gateway, Clock clock) {
        this.gateway = Objects.requireNonNull(gateway);
        this.clock = Objects.requireNonNull(clock);
    }

    public void login(LoginCredentials credentials) throws AuthenticationException {
        LoginCredentials normalized = validateCredentials(credentials);
        synchronized (operationLock) {
            SessionTokens tokens = gateway.login(normalized);
            installSession(new SignedInSession(normalized.email(), tokens));
        }
    }

    public void signInFederated(SessionTokens tokens) throws AuthenticationException {
        synchronized (operationLock) {
            installSession(new SignedInSession(null, Objects.requireNonNull(tokens)));
        }
    }

    public void logout() throws AuthenticationException {
        synchronized (operationLock) {
            SignedInSession current = session;
            if (current == null) {
                return;
            }
            gateway.logout(current.tokens().refreshToken());
            session = null;
        }
    }

    public boolean isAuthenticated() {
        SignedInSession current = session;
        return current != null && clock.instant().isBefore(current.tokens().expiresAt());
    }

    public Optional<String> email() {
        SignedInSession current = session;
        return current == null ? Optional.empty() : Optional.ofNullable(current.email());
    }

    public Optional<String> accountId() {
        SignedInSession current = session;
        return current == null
                ? Optional.empty()
                : Optional.of(current.tokens().identity().accountId().toString());
    }

    public String accessToken() throws AuthenticationException {
        synchronized (operationLock) {
            SignedInSession current = requireSession();
            if (clock.instant().plusSeconds(30).isBefore(current.tokens().access().expiresAt())) {
                return current.tokens().access().value();
            }
            return refreshSession(current);
        }
    }

    public String renewAfterRejection(String rejectedToken) throws AuthenticationException {
        synchronized (operationLock) {
            SignedInSession current = requireSession();
            if (!current.tokens().access().value().equals(rejectedToken)) {
                return accessToken();
            }
            return refreshSession(current);
        }
    }

    @FunctionalInterface
    public interface AuthenticatedOperation<E extends Exception> {
        void run(String accessToken) throws E;
    }

    public <E extends Exception> void runAuthenticated(AuthenticatedOperation<E> operation)
            throws AuthenticationException, E {
        synchronized (operationLock) {
            operation.run(accessToken());
        }
    }

    private String refreshSession(SignedInSession current) throws AuthenticationException {
        try {
            SessionTokens renewed = gateway.refresh(current.tokens().refreshToken());
            if (!renewed.identity().equals(current.tokens().identity())
                    || !renewed.expiresAt().equals(current.tokens().expiresAt())) {
                throw new AuthenticationException(AuthenticationError.UNEXPECTED);
            }
            installSession(new SignedInSession(current.email(), renewed));
            return renewed.access().value();
        } catch (AuthenticationException failure) {
            if (failure.kind() != AuthenticationError.UNAVAILABLE
                    && failure.kind() != AuthenticationError.RATE_LIMITED) {
                session = null;
            }
            throw failure;
        }
    }

    private SignedInSession requireSession() throws AuthenticationException {
        SignedInSession current = session;
        if (current == null || !clock.instant().isBefore(current.tokens().expiresAt())) {
            session = null;
            throw new AuthenticationException(AuthenticationError.NO_SESSION);
        }
        return current;
    }

    private void installSession(SignedInSession signedIn) throws AuthenticationException {
        synchronized (stateLock) {
            if (closed
                    || !clock.instant().isBefore(signedIn.tokens().expiresAt())
                    || !clock.instant().isBefore(signedIn.tokens().access().expiresAt())) {
                throw new AuthenticationException(AuthenticationError.NO_SESSION);
            }
            session = signedIn;
        }
    }

    private LoginCredentials validateCredentials(LoginCredentials credentials)
            throws AuthenticationException {
        if (credentials == null || credentials.email() == null || credentials.password() == null) {
            throw new AuthenticationException(AuthenticationError.INVALID_INPUT);
        }
        String email = RegistrationValidator.normalizeEmail(credentials.email());
        String password = credentials.password();
        if (!RegistrationValidator.isValidEmailFormat(email)
                || password.isEmpty()
                || password.codePointCount(0, password.length()) > 128) {
            throw new AuthenticationException(AuthenticationError.INVALID_INPUT);
        }
        return new LoginCredentials(email, password);
    }

    public void invalidate() {
        synchronized (stateLock) {
            session = null;
        }
    }

    @Override
    public void close() {
        synchronized (stateLock) {
            closed = true;
            session = null;
        }
    }
}
