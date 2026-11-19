package mx.uv.sonimbus.desktop;

import javafx.stage.Stage;

import mx.uv.sonimbus.desktop.data.api.ApiConfig;
import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;
import mx.uv.sonimbus.desktop.data.api.auth.google.HttpGoogleGateway;
import mx.uv.sonimbus.desktop.data.api.auth.password.HttpPasswordGateway;
import mx.uv.sonimbus.desktop.data.api.auth.registration.HttpRegistrationGateway;
import mx.uv.sonimbus.desktop.data.api.auth.session.HttpAuthenticationGateway;
import mx.uv.sonimbus.desktop.data.browser.google.DesktopGoogleBrowser;
import mx.uv.sonimbus.desktop.domain.google.GoogleService;
import mx.uv.sonimbus.desktop.domain.password.PasswordService;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationService;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.navigation.ControllerFactory;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;
import mx.uv.sonimbus.desktop.ui.theme.ThemeManager;

/**
 * Contenedor de la aplicación: crea los servicios una sola vez y los registra en la fábrica de
 * controladores. No hay singletons estáticos; quien necesita algo lo recibe por constructor.
 */
final class AppContext implements AutoCloseable {

    private final BackgroundTasks backgroundTasks = new BackgroundTasks();
    private final ViewManager viewManager;
    private final SessionService sessions;
    private final GoogleService google;

    AppContext(Stage stage, ApiConfig config) {
        this(stage, config, false);
    }

    AppContext(Stage stage, ApiConfig config, boolean demo) {
        AuthApi authApi = new AuthApi(config);
        sessions = new SessionService(new HttpAuthenticationGateway(authApi));
        google =
                new GoogleService(
                        new HttpGoogleGateway(authApi), new DesktopGoogleBrowser(), sessions);
        ControllerFactory controllers = new ControllerFactory();
        viewManager =
                new ViewManager(
                        stage,
                        controllers,
                        new ThemeManager(),
                        () -> demo || sessions.isAuthenticated());
        controllers
                .register(ApiConfig.class, config)
                .register(
                        RegistrationService.class,
                        new RegistrationService(new HttpRegistrationGateway(authApi)))
                .register(
                        PasswordService.class,
                        new PasswordService(new HttpPasswordGateway(authApi), sessions))
                .register(GoogleService.class, google)
                .register(SessionService.class, sessions)
                .register(BackgroundTasks.class, backgroundTasks)
                .register(ViewManager.class, viewManager);
    }

    ViewManager viewManager() {
        return viewManager;
    }

    @Override
    public void close() {
        google.close();
        sessions.close();
        backgroundTasks.close();
    }
}
