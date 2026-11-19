package mx.uv.sonimbus.desktop.domain.google;

import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.domain.session.SessionTokens;

import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.NoSuchAlgorithmException;
import java.security.SecureRandom;
import java.time.Instant;
import java.util.Base64;
import java.util.Objects;
import java.util.Optional;
import java.util.concurrent.atomic.AtomicBoolean;

public final class GoogleService implements AutoCloseable {
    private final GoogleGateway gateway;
    private final GoogleBrowser browser;
    private final SessionService sessions;
    private final AtomicBoolean running = new AtomicBoolean();
    private final Object completionLock = new Object();
    private volatile AtomicBoolean cancellation = new AtomicBoolean();

    public GoogleService(GoogleGateway gateway, GoogleBrowser browser, SessionService sessions) {
        this.gateway = Objects.requireNonNull(gateway);
        this.browser = Objects.requireNonNull(browser);
        this.sessions = Objects.requireNonNull(sessions);
    }

    public void signIn() throws GoogleException {
        AtomicBoolean cancelled = startFlow();
        try {
            String verifier = newVerifier();
            GoogleAttempt attempt = gateway.createAttempt(challenge(verifier));
            GoogleExchange proof = authorize(attempt, verifier, cancelled);
            SessionTokens tokens = gateway.exchange(proof);
            synchronized (completionLock) {
                checkCancellation(cancelled);
                sessions.signInFederated(tokens);
            }
        } catch (AuthenticationException invalidSession) {
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        } finally {
            finishFlow();
        }
    }

    public void link() throws GoogleException {
        link(null);
    }

    public void link(String password) throws GoogleException {
        if (password != null
                && (password.isEmpty() || password.codePointCount(0, password.length()) > 128)) {
            throw new GoogleException(GoogleError.INVALID_PASSWORD);
        }
        AtomicBoolean cancelled = startFlow();
        try {
            if (password != null) {
                withSession(
                        token -> {
                            gateway.reauthenticate(token, password);
                            return null;
                        });
            }
            checkCancellation(cancelled);
            String verifier = newVerifier();
            GoogleAttempt attempt =
                    withSession(token -> gateway.createLinkAttempt(token, challenge(verifier)));
            GoogleExchange proof = authorize(attempt, verifier, cancelled);
            gateway.exchangeLink(proof);
        } finally {
            finishFlow();
        }
    }

    public Optional<GoogleConnection> connection() throws GoogleException {
        return withSession(gateway::connection);
    }

    @FunctionalInterface
    private interface SessionOperation<T> {
        T run(String accessToken) throws GoogleException;
    }

    private <T> T withSession(SessionOperation<T> operation) throws GoogleException {
        try {
            return operation.run(sessions.accessToken());
        } catch (AuthenticationException failure) {
            GoogleError kind =
                    switch (failure.kind()) {
                        case RATE_LIMITED -> GoogleError.RATE_LIMITED;
                        case UNAVAILABLE -> GoogleError.UNAVAILABLE;
                        default -> GoogleError.NO_SESSION;
                    };
            throw new GoogleException(kind, failure.retryAfter().orElse(null), failure);
        } catch (GoogleException failure) {
            if (failure.kind() == GoogleError.NO_SESSION) {
                sessions.invalidate();
            }
            throw failure;
        }
    }

    private AtomicBoolean startFlow() throws GoogleException {
        synchronized (completionLock) {
            if (!running.compareAndSet(false, true)) {
                throw new GoogleException(GoogleError.BUSY);
            }
            AtomicBoolean cancelled = new AtomicBoolean();
            cancellation = cancelled;
            return cancelled;
        }
    }

    private String newVerifier() {
        byte[] random = new byte[32];
        new SecureRandom().nextBytes(random);
        return Base64.getUrlEncoder().withoutPadding().encodeToString(random);
    }

    private GoogleExchange authorize(
            GoogleAttempt attempt, String verifier, AtomicBoolean cancelled)
            throws GoogleException {
        if (!Instant.now().isBefore(attempt.expiresAt())) {
            throw new GoogleException(GoogleError.EXPIRED);
        }
        checkCancellation(cancelled);
        String completion = browser.authorize(attempt, cancelled::get);
        checkCancellation(cancelled);
        return new GoogleExchange(
                attempt.attemptId(), attempt.attemptSecret(), completion, verifier);
    }

    private void finishFlow() {
        browser.cancel();
        running.set(false);
    }

    private String challenge(String verifier) throws GoogleException {
        try {
            return Base64.getUrlEncoder()
                    .withoutPadding()
                    .encodeToString(
                            MessageDigest.getInstance("SHA-256")
                                    .digest(verifier.getBytes(StandardCharsets.US_ASCII)));
        } catch (NoSuchAlgorithmException unavailable) {
            throw new GoogleException(GoogleError.UNAVAILABLE);
        }
    }

    private void checkCancellation(AtomicBoolean cancelled) throws GoogleException {
        if (cancelled.get()) {
            throw new GoogleException(GoogleError.CANCELLED);
        }
    }

    public void cancel() {
        synchronized (completionLock) {
            cancellation.set(true);
        }
        browser.cancel();
    }

    @Override
    public void close() {
        cancel();
        browser.close();
    }
}
