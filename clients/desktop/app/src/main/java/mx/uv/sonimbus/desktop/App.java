package mx.uv.sonimbus.desktop;

import javafx.application.Application;
import javafx.stage.Stage;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

public class App extends Application {

    @Override
    public void start(Stage stage) {
        ViewManager viewManager = new ViewManager(stage);
        viewManager.show(View.LOGIN);
        stage.setTitle("Sonimbus");
        stage.show();
    }

    public static void main(String[] args) {
        launch(args);
    }
}
