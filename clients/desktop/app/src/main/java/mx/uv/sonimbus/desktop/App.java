package mx.uv.sonimbus.desktop;

import java.util.List;
import java.util.Locale;
import javafx.application.Application;
import javafx.stage.Stage;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

public class App extends Application {

    @Override
    public void start(Stage stage) {
        ViewManager viewManager = new ViewManager(stage);
        viewManager.show(initialView());
        stage.setTitle("Sonimbus");
        stage.show();
    }

    public static void main(String[] args) {
        launch(args);
    }

    /**
     * TEMPORAL: mientras no exista el inicio de sesión (CU-03), el argumento {@code MAIN}
     * abre directamente la ventana principal para poder recorrer sus vistas.
     */
    private View initialView() {
        List<String> args = getParameters().getRaw();
        if (!args.isEmpty() && args.get(0).toUpperCase(Locale.ROOT).equals(View.MAIN.name())) {
            return View.MAIN;
        }
        return View.LOGIN;
    }
}
