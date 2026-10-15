package mx.uv.sonimbus.desktop.dev;

import java.net.URL;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.atomic.AtomicInteger;

import javafx.application.Platform;
import javafx.fxml.FXMLLoader;
import javafx.scene.Parent;
import javafx.scene.Scene;
import mx.uv.sonimbus.desktop.ui.navigation.View;

/**
 * Prueba de humo TEMPORAL: carga cada FXML del enum View con su CSS y
 * reporta los que fallan (propiedades inexistentes, imports, controladores).
 * No forma parte de la app: se copia a src/main/java/mx/uv/sonimbus/desktop/dev/,
 * se ejecuta y se borra.
 */
public final class FxmlSmokeCheck {

    private static final String UI = "/mx/uv/sonimbus/desktop/ui/";
    private static final String CSS = "/mx/uv/sonimbus/desktop/css/theme.css";

    public static void main(String[] args) throws Exception {
        AtomicInteger fails = new AtomicInteger();
        CountDownLatch done = new CountDownLatch(1);
        Platform.startup(() -> { });
        Platform.runLater(() -> {
            URL css = FxmlSmokeCheck.class.getResource(CSS);
            for (View view : View.values()) {
                String path = UI + view.fxml();
                try {
                    URL url = FxmlSmokeCheck.class.getResource(path);
                    if (url == null) {
                        throw new IllegalStateException("no existe " + path);
                    }
                    Parent root = FXMLLoader.load(url);
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
