package mx.uv.sonimbus.desktop;

import java.io.IOException;
import java.io.UncheckedIOException;
import java.util.EnumSet;
import java.util.List;
import java.util.Locale;
import java.util.Set;
import java.util.prefs.Preferences;
import javafx.application.Application;
import javafx.fxml.FXMLLoader;
import javafx.scene.Parent;
import javafx.scene.Scene;
import javafx.scene.control.Label;
import javafx.scene.layout.StackPane;
import javafx.stage.Stage;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

public class App extends Application {

    @Override
    public void start(Stage stage) {
        View view = requestedView();
        if (view == View.LOGIN) {
            new ViewManager(stage).show(View.LOGIN);
            stage.setTitle("Sonimbus");
        } else {
            showForDemo(stage, view);
        }
        stage.show();
    }

    public static void main(String[] args) {
        launch(args);
    }



    private static final String UI_ROOT = "/mx/uv/sonimbus/desktop/ui/";
    private static final String DEMO_INDEX_KEY = "demoViewIndex";

    /** Vistas que ocupan toda la ventana; las demás se muestran dentro del shell. */
    private static final Set<View> FULL_WINDOW = EnumSet.of(
            View.LOGIN, View.REGISTER, View.VERIFY_ACCOUNT, View.RECOVER_PASSWORD, View.MAIN);

    /** Diálogos modales: se muestran solos, con su tamaño preferido. */
    private static final Set<View> DIALOGS = EnumSet.of(
            View.CREATE_GROUP, View.JOIN_GROUP, View.TASK_DETAIL);

    private View requestedView() {
        List<String> args = getParameters().getRaw();
        if (args.isEmpty()) {
            return View.LOGIN;
        }
        View[] views = View.values();
        String arg = args.get(0).trim();
        if (arg.equalsIgnoreCase("siguiente")) {
            Preferences prefs = Preferences.userNodeForPackage(App.class);
            int index = Math.floorMod(prefs.getInt(DEMO_INDEX_KEY, 0), views.length);
            prefs.putInt(DEMO_INDEX_KEY, index + 1);
            return views[index];
        }
        try {
            int number = Integer.parseInt(arg);
            if (number >= 1 && number <= views.length) {
                return views[number - 1];
            }
        } catch (NumberFormatException e) {

        }
        try {
            return View.valueOf(arg.toUpperCase(Locale.ROOT).replace('-', '_'));
        } catch (IllegalArgumentException e) {
            System.err.println("Vista desconocida: " + arg + ". Opciones:");
            for (View view : views) {
                System.err.println("  " + (view.ordinal() + 1) + "  " + view);
            }
            return View.LOGIN;
        }
    }

    private void showForDemo(Stage stage, View view) {
        Parent root = load(view);
        Scene scene;
        if (DIALOGS.contains(view)) {
            scene = new Scene(root);
        } else if (FULL_WINDOW.contains(view)) {
            scene = new Scene(root, 1280, 800);
        } else {
            Parent shell = load(View.MAIN);
            ((StackPane) shell.lookup("#contentPane")).getChildren().setAll(root);
            ((Label) shell.lookup("#sectionTitleLabel")).setText(demoTitle(view));
            scene = new Scene(shell, 1280, 800);
        }
        stage.setScene(scene);
        stage.setTitle("Sonimbus · " + (view.ordinal() + 1) + " de " + View.values().length
                + " · " + view);
    }

    private static String demoTitle(View view) {
        return switch (view) {
            case TEXT_CHANNEL -> "# general";
            case VOICE_CHANNEL -> "Canal de voz";
            case SHARED_FILES -> "Archivos compartidos";
            case GROUP_SETTINGS -> "Configuración del grupo";
            case PROJECTS -> "Proyectos y tareas";
            case REPORTS -> "Reportes";
            case NOTIFICATIONS -> "Notificaciones";
            case PROFILE -> "Mi perfil";
            default -> "";
        };
    }

    private static Parent load(View view) {
        try {
            return FXMLLoader.load(App.class.getResource(UI_ROOT + view.fxml()));
        } catch (IOException e) {
            throw new UncheckedIOException("No se pudo cargar la vista " + view, e);
        }
    }
}
