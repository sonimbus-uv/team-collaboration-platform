package mx.uv.sonimbus.desktop;

import java.util.List;
import java.util.Optional;
import java.util.prefs.Preferences;
import javafx.application.Application;
import javafx.application.Platform;
import javafx.stage.Stage;
import mx.uv.sonimbus.desktop.data.api.ApiConfig;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

public class App extends Application {

    private static final String DEMO_INDEX_KEY = "demoViewIndex";

    private AppContext context;

    @Override
    public void start(Stage stage) {
        context = new AppContext(stage, ApiConfig.fromEnvironment());
        List<String> args = getParameters().getRaw();
        Optional<View> demoView = DemoNavigation.requestedView(args, App::nextDemoIndex);
        if (demoView.isEmpty() && !args.isEmpty() && !args.get(0).isBlank()) {
            System.err.println(DemoNavigation.usage(args.get(0)));
        }
        show(stage, context.viewManager(), demoView.orElse(View.LOGIN), demoView.isPresent());
    }

    @Override
    public void stop() {
        if (context != null) {
            context.close();
        }
    }

    public static void main(String[] args) {
        launch(args);
    }

    private static void show(Stage stage, ViewManager viewManager, View view, boolean demo) {
        switch (DemoNavigation.placementOf(view)) {
            case FULL_WINDOW -> viewManager.show(view);
            case CONTENT -> {
                viewManager.show(View.MAIN);
                viewManager.showContent(view);
            }
            case DIALOG -> viewManager.show(View.MAIN);
        }
        stage.setTitle(demo ? DemoNavigation.windowTitle(view) : "Sonimbus");
        stage.show();
        if (DemoNavigation.placementOf(view) == DemoNavigation.Placement.DIALOG) {
            // Después de mostrar la ventana, para que el diálogo quede encima del shell.
            Platform.runLater(() -> viewManager.showDialog(view, DemoNavigation.dialogTitle(view)));
        }
    }

    /** Índice de {@code siguiente}; avanza el contador guardado para la próxima ejecución. */
    private static int nextDemoIndex() {
        Preferences prefs = Preferences.userNodeForPackage(App.class);
        int index = Math.floorMod(prefs.getInt(DEMO_INDEX_KEY, 0), View.values().length);
        prefs.putInt(DEMO_INDEX_KEY, index + 1);
        return index;
    }
}
