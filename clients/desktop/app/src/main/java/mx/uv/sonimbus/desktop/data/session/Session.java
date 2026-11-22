package mx.uv.sonimbus.desktop.data.session;

import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationError;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.domain.session.SessionService;

public final class Session {
    private final SessionService sessions;
    private final Runnable onExpired;

    public Session(SessionService sessions, Runnable onExpired) {
        this.sessions = sessions;
        this.onExpired = onExpired;
    }

    public String accessToken() throws ApiException {
        try {
            return sessions.accessToken();
        } catch (AuthenticationException failure) {
            throw translate(failure);
        }
    }

    public String renewAfterRejection(String rejectedToken) throws ApiException {
        try {
            return sessions.renewAfterRejection(rejectedToken);
        } catch (AuthenticationException failure) {
            throw translate(failure);
        }
    }

    private ApiException translate(AuthenticationException failure) {
        if (!sessions.isAuthenticated()) {
            onExpired.run();
        }
        if (failure.kind() == AuthenticationError.NO_SESSION) {
            return new ApiException(ApiErrorKind.UNAUTHORIZED, "session", 401, null);
        }
        if (failure.getCause() instanceof ApiException apiFailure) {
            return apiFailure;
        }
        return new ApiException(ApiErrorKind.UNEXPECTED, "session", null);
    }
}
