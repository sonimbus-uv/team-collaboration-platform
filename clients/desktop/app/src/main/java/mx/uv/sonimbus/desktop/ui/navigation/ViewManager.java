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
import javafx.util.Callback;

/**
 * Navegación del cliente: vistas de ventana completa, vistas dentro del shell y diálogos.
 *
 * <p>Cada método tiene una variante con parámetros, que recibe el controlador destino si
 * implementa {@link Parameterized}.
 */
public final class ViewManager {

    static final String UI_ROOT = "/mx/uv/sonimbus/desktop/ui/";
    private static final String THEME = "/mx/uv/sonimbus/desktop/css/theme.css";

    private final Stage stage;
    private final Callback<Class<?>, Object> controllerFactory;
    private ContentHost contentHost;

    public ViewManager(Stage stage, Callback<Class<?>, Object> controllerFactory) {
        this.stage = Objects.requireNonNull(stage);
        this.controllerFactory = Objects.requireNonNull(controllerFactory);
    }

    /** Reemplaza todo el contenido de la ventana principal. */
    public void show(View view) {
        show(view, null);
    }

    public void show(View view, Object parameters) {
        FXMLLoader loader = load(view, parameters);
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
        showContent(view, null);
    }

    public void showContent(View view, Object parameters) {
        ContentHost host = requireContentHost();
        host.setContent(view, load(view, parameters).getRoot());
    }

    /** Quita la vista del área central de la vista actual. */
    public void closeContent() {
        requireContentHost().clearContent();
    }

    /** Abre la vista en una ventana modal y espera a que se cierre. */
    public void showDialog(View view, String title) {
        showDialog(view, title, null);
    }

    public void showDialog(View view, String title, Object parameters) {
        Scene scene = new Scene(load(view, parameters).getRoot());
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

    private FXMLLoader load(View view, Object parameters) {
        FXMLLoader loader = new FXMLLoader(resource(UI_ROOT + view.fxml()));
        loader.setControllerFactory(controllerFactory);
        try {
            loader.load();
        } catch (IOException e) {
            throw new UncheckedIOException("No se pudo cargar la vista " + view, e);
        }
        Object controller = loader.getController();
        if (controller instanceof Navigable navigable) {
            navigable.setViewManager(this);
        }
        deliverParameters(view, controller, parameters);
        return loader;
    }

    /** Entrega {@code parameters} al controlador; falla con un mensaje claro si no los acepta. */
    static void deliverParameters(View view, Object controller, Object parameters) {
        if (parameters == null) {
            return;
        }
        if (!(controller instanceof Parameterized<?> target)) {
            throw new IllegalArgumentException("La vista " + view + " no recibe parámetros");
        }
        deliver(view, target, parameters);
    }

    private static <P> void deliver(View view, Parameterized<P> target, Object parameters) {
        Class<P> type = target.parametersType();
        if (!type.isInstance(parameters)) {
            throw new IllegalArgumentException("La vista " + view + " espera " + type.getSimpleName()
                    + ", no " + parameters.getClass().getSimpleName());
        }
        target.setParameters(type.cast(parameters));
    }

    private static URL resource(String path) {
        return Objects.requireNonNull(
                ViewManager.class.getResource(path), () -> "Recurso no encontrado: " + path);
    }
}
