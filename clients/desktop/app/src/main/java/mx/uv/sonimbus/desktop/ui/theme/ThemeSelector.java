package mx.uv.sonimbus.desktop.ui.theme;

import javafx.scene.control.ComboBox;

public final class ThemeSelector extends ComboBox<Theme> {

    public ThemeSelector() {
        getItems().setAll(Theme.values());
        setValue(Theme.LIGHT);
        setAccessibleText("Apariencia");
        setAccessibleHelp("Elige el tema Claro u Oscuro.");
        getStyleClass().add("theme-selector");
        setPrefWidth(128);
        sceneProperty()
                .addListener(
                        (observable, previous, current) -> {
                            if (current != null) {
                                ThemeManager.installSelector(this);
                            }
                        });
    }
}
