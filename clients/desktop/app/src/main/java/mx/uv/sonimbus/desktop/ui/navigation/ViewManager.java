package mx.uv.sonimbus.desktop.ui.navigation;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.net.URL;
import java.util.Objects;
import javafx.fxml.FXMLLoader;
import javafx.scene.Parent;
import javafx.scene.Scene;
import javafx.stage.Modality;
import javafx.stage.Stage;

/** Navegación del cliente: vistas de ventana completa, vistas dentro del shell y diálogos. */
public final class ViewManager {

    static final String UI_ROOT = "/mx/uv/sonimbus/desktop/ui/";
    private static final String THEME = "/mx/uv/sonimbus/desktop/css/theme.css";

    private final Stage stage;
    private ContentHost contentHost;

    public ViewManager(Stage stage) {
        this.stage = Objects.requireNonNull(stage);
    }

    /** Reemplaza todo el contenido de la ventana principal. */
    public void show(View view) {
        FXMLLoader loader = load(view);
        Parent root = loader.getRoot();
        contentHost = loader.getController() instanceof ContentHost host ? host : null;
        if (stage.getScene() == null) {
            Scene scene = new Scene(root, 1280, 800);
            scene.getStylesheets().add(resource(THEME).toExternalForm());
            stage.setScene(scene);
        } else {
            stage.getScene().setRoot(root);
        }
    }

    /** Muestra la vista en el área central de la vista actual, que debe ser un {@link ContentHost}. */
    public void showContent(View view) {
        requireContentHost().setContent(load(view).getRoot());
    }

    /** Quita la vista del área central de la vista actual. */
    public void closeContent() {
        requireContentHost().clearContent();
    }

    /** Abre la vista en una ventana modal y espera a que se cierre. */
    public void showDialog(View view, String title) {
        Scene scene = new Scene(load(view).getRoot());
        scene.getStylesheets().add(resource(THEME).toExternalForm());
        Stage dialog = new Stage();
        dialog.initOwner(stage);
        dialog.initModality(Modality.WINDOW_MODAL);
        dialog.setTitle(title);
        dialog.setResizable(false);
        dialog.setScene(scene);
        dialog.showAndWait();
    }

    private ContentHost requireContentHost() {
        if (contentHost == null) {
            throw new IllegalStateException("La vista actual no tiene área de contenido");
        }
        return contentHost;
    }

    private FXMLLoader load(View view) {
        FXMLLoader loader = new FXMLLoader(resource(UI_ROOT + view.fxml()));
        try {
            loader.load();
        } catch (IOException e) {
            throw new UncheckedIOException("No se pudo cargar la vista " + view, e);
        }
        if (loader.getController() instanceof Navigable navigable) {
            navigable.setViewManager(this);
        }
        return loader;
    }

    private static URL resource(String path) {
        return Objects.requireNonNull(
                ViewManager.class.getResource(path), () -> "Recurso no encontrado: " + path);
    }
}
