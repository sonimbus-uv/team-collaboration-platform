package mx.uv.sonimbus.desktop.dev;

import java.net.URI;
import java.net.URL;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.atomic.AtomicInteger;

import javafx.application.Platform;
import javafx.fxml.FXMLLoader;
import javafx.scene.Parent;
import javafx.scene.Scene;
import javafx.stage.Stage;
import mx.uv.sonimbus.desktop.data.api.ApiConfig;
import mx.uv.sonimbus.desktop.data.api.AuthApi;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.navigation.ControllerFactory;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

/**
 * Prueba de humo TEMPORAL: carga cada FXML del enum View con su CSS y
 * reporta los que fallan (propiedades inexistentes, imports, controladores).
 * No forma parte de la app: se copia a src/main/java/mx/uv/sonimbus/desktop/dev/,
 * se ejecuta y se borra.
 *
 * Los controladores reciben sus dependencias por constructor, así que se cargan con la
 * misma ControllerFactory que usa la app (AppContext). La URL de Auth apunta a un puerto
 * cerrado: la prueba no hace peticiones.
 */
public final class FxmlSmokeCheck {

    private static final String UI = "/mx/uv/sonimbus/desktop/ui/";
    private static final String CSS = "/mx/uv/sonimbus/desktop/css/theme.css";

    public static void main(String[] args) throws Exception {
        AtomicInteger fails = new AtomicInteger();
        CountDownLatch done = new CountDownLatch(1);
        Platform.startup(() -> { });
        Platform.runLater(() -> {
            ControllerFactory controllers = new ControllerFactory();
            ApiConfig config = ApiConfig.withAuthUrl(URI.create("http://127.0.0.1:9"));
            controllers
                    .register(ApiConfig.class, config)
                    .register(AuthApi.class, new AuthApi(config))
                    .register(BackgroundTasks.class, new BackgroundTasks())
                    .register(ViewManager.class, new ViewManager(new Stage(), controllers));
            URL css = FxmlSmokeCheck.class.getResource(CSS);
            for (View view : View.values()) {
                String path = UI + view.fxml();
                try {
                    URL url = FxmlSmokeCheck.class.getResource(path);
                    if (url == null) {
                        throw new IllegalStateException("no existe " + path);
                    }
                    FXMLLoader loader = new FXMLLoader(url);
                    loader.setControllerFactory(controllers);
                    Parent root = loader.load();
                    Scene scene = new Scene(root, 1280, 800);
                    if (css != null) {
                        scene.getStylesheets().add(css.toExternalForm());
                    }
                    root.applyCss();
                    root.layout();
                    System.out.println("OK   " + view + "  (" + view.fxml() + ")");
                } catch (Exception e) {
                    fails.incrementAndGet();
                    System.out.println("ERR  " + view + "  (" + view.fxml() + "): " + e);
                    for (Throwable c = e.getCause(); c != null; c = c.getCause()) {
                        System.out.println("       causa: " + c);
                    }
                }
            }
            done.countDown();
        });
        done.await();
        Platform.exit();
        System.out.println(fails.get() == 0 ? "\nTodas las vistas cargan." : "\nFallaron " + fails.get() + " vista(s).");
        System.exit(fails.get() == 0 ? 0 : 1);
    }
}
