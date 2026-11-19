package mx.uv.sonimbus.desktop;

import javafx.application.Platform;
import javafx.stage.Stage;
import mx.uv.sonimbus.desktop.data.api.ApiConfig;
import mx.uv.sonimbus.desktop.data.api.AuthApi;
import mx.uv.sonimbus.desktop.data.core.CoreApi;
import mx.uv.sonimbus.desktop.data.realtime.RealtimeClient;
import mx.uv.sonimbus.desktop.data.session.Session;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.navigation.ControllerFactory;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

/**
 * Contenedor de la aplicación: crea los servicios una sola vez y los registra en la fábrica de
 * controladores. No hay singletons estáticos; quien necesita algo lo recibe por constructor.
 */
final class AppContext implements AutoCloseable {

    private final BackgroundTasks backgroundTasks = new BackgroundTasks();
    private final ViewManager viewManager;

    AppContext(Stage stage, ApiConfig config) {
        ControllerFactory controllers = new ControllerFactory();
        viewManager = new ViewManager(stage, controllers);
        AuthApi authApi = new AuthApi(config);
        Session session = new Session(authApi::refresh);
        // Auth ya no renueva la sesión (venció a los 20 días o se abrió otra en escritorio, D-12).
        session.setOnExpired(() -> Platform.runLater(() -> viewManager.show(View.LOGIN)));
        controllers
                .register(ApiConfig.class, config)
                .register(AuthApi.class, authApi)
                .register(Session.class, session)
                .register(CoreApi.class, new CoreApi(config, session))
                .register(RealtimeClient.class, new RealtimeClient(config, session))
                .register(BackgroundTasks.class, backgroundTasks)
                .register(ViewManager.class, viewManager);
    }

    ViewManager viewManager() {
        return viewManager;
    }

    @Override
    public void close() {
        backgroundTasks.close();
    }
}
