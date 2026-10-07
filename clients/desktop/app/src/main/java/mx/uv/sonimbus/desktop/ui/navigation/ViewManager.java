package mx.uv.sonimbus.desktop.ui.navigation;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.net.URL;
import java.util.Objects;
import javafx.fxml.FXMLLoader;
import javafx.scene.Parent;
import javafx.scene.Scene;
import javafx.stage.Stage;

/** Cambia la vista que se muestra dentro de la ventana principal. */
public final class ViewManager {

    static final String UI_ROOT = "/mx/uv/sonimbus/desktop/ui/";
    private static final String THEME = "/mx/uv/sonimbus/desktop/css/theme.css";

    private final Stage stage;

    public ViewManager(Stage stage) {
        this.stage = Objects.requireNonNull(stage);
    }

    public void show(View view) {
        Parent root = load(view);
        if (stage.getScene() == null) {
            Scene scene = new Scene(root, 1280, 800);
            scene.getStylesheets().add(resource(THEME).toExternalForm());
            stage.setScene(scene);
        } else {
            stage.getScene().setRoot(root);
        }
    }

    private Parent load(View view) {
        try {
            return FXMLLoader.load(resource(UI_ROOT + view.fxml()));
        } catch (IOException e) {
            throw new UncheckedIOException("No se pudo cargar la vista " + view, e);
        }
    }

    private static URL resource(String path) {
        return Objects.requireNonNull(
                ViewManager.class.getResource(path), () -> "Recurso no encontrado: " + path);
    }
}
