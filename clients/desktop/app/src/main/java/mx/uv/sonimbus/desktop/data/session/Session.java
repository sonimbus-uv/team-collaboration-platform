package mx.uv.sonimbus.desktop.data.session;

import java.time.Clock;
import java.time.Duration;
import java.time.Instant;
import java.util.Objects;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.Tokens;

/**
 * Sesión con Auth, solo en memoria: al cerrar la aplicación hay que iniciar sesión de nuevo.
 * Renueva el access token cuando le queda menos de {@link #REFRESH_MARGIN} o cuando Core lo
 * rechaza. Es segura entre hilos: las llamadas a los servicios se hacen en segundo plano.
 */
public final class Session {

    /** Renueva el token antes de que venza, para no enviar uno a punto de caducar. */
    static final Duration REFRESH_MARGIN = Duration.ofSeconds(60);

    /** Renovación del par de tokens; en la aplicación es {@code AuthApi::refresh}. */
    @FunctionalInterface
    public interface Refresher {
        Tokens refresh(String refreshToken) throws ApiException;
    }

    private final Refresher refresher;
    private final Clock clock;
    private Runnable onExpired = () -> { };

    private Tokens tokens;
    private Instant accessExpiresAt;

    public Session(Refresher refresher) {
        this(refresher, Clock.systemUTC());
    }

    Session(Refresher refresher, Clock clock) {
        this.refresher = Objects.requireNonNull(refresher);
        this.clock = Objects.requireNonNull(clock);
    }

    /** Se ejecuta, en el hilo que detectó el problema, cuando Auth ya no renueva la sesión. */
    public synchronized void setOnExpired(Runnable onExpired) {
        this.onExpired = Objects.requireNonNull(onExpired);
    }

    public synchronized void start(Tokens tokens) {
        store(Objects.requireNonNull(tokens));
    }

    public synchronized void clear() {
        tokens = null;
        accessExpiresAt = null;
    }

    public synchronized boolean isActive() {
        return tokens != null;
    }

    /** {@code auth.accounts.id} del usuario: es el {@code user_id} y el {@code author_id} en Core. */
    public synchronized String userId() throws ApiException {
        return requireTokens().accountId();
    }

    /** Access token vigente; lo renueva antes si está por vencer. */
    public synchronized String accessToken() throws ApiException {
        requireTokens();
        if (clock.instant().plus(REFRESH_MARGIN).isAfter(accessExpiresAt)) {
            renew();
        }
        return tokens.accessToken();
    }

    /**
     * Core rechazó {@code rejected}. Si otro hilo no lo renovó ya, se renueva ahora. Devuelve el
     * token que hay que usar en el reintento.
     */
    public synchronized String renewAfterRejection(String rejected) throws ApiException {
        requireTokens();
        if (tokens.accessToken().equals(rejected)) {
            renew();
        }
        return tokens.accessToken();
    }

    private void renew() throws ApiException {
        try {
            store(refresher.refresh(tokens.refreshToken()));
        } catch (ApiException e) {
            if (e.kind() == ApiErrorKind.UNAUTHORIZED) {
                clear();
                onExpired.run();
            }
            throw e;
        }
    }

    private void store(Tokens next) {
        tokens = next;
        accessExpiresAt = clock.instant().plusSeconds(next.accessExpiresIn());
    }

    private Tokens requireTokens() throws ApiException {
        if (tokens == null) {
            throw new ApiException(ApiErrorKind.UNAUTHORIZED, "session", 401, null);
        }
        return tokens;
    }
}
