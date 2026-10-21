package mx.uv.sonimbus.desktop;

import javafx.stage.Stage;
import mx.uv.sonimbus.desktop.data.api.ApiConfig;
import mx.uv.sonimbus.desktop.data.api.AuthApi;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.navigation.ControllerFactory;
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
        controllers
                .register(ApiConfig.class, config)
                .register(AuthApi.class, new AuthApi(config))
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
