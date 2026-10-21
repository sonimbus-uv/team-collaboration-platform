package mx.uv.sonimbus.desktop.ui.common;

import javafx.scene.control.Button;
import javafx.scene.control.ProgressIndicator;

/** Botón que muestra un indicador y queda deshabilitado mientras una petición está en curso. */
public final class BusyButton {

    private static final String IDLE_TEXT = "sonimbus.idleText";
    private static final String IDLE_ACCESSIBLE_TEXT = "sonimbus.idleAccessibleText";

    private BusyButton() {
    }

    public static void setBusy(Button button, boolean busy, String busyText) {
        if (busy) {
            if (!button.getProperties().containsKey(IDLE_TEXT)) {
                button.getProperties().put(IDLE_TEXT, button.getText());
                button.getProperties().put(IDLE_ACCESSIBLE_TEXT, button.getAccessibleText());
            }
            ProgressIndicator spinner = new ProgressIndicator();
            spinner.getStyleClass().add("button-spinner");
            spinner.setFocusTraversable(false);
            button.setGraphic(spinner);
            button.setText(busyText);
            button.setAccessibleText(busyText);
            button.setDisable(true);
        } else if (button.getProperties().containsKey(IDLE_TEXT)) {
            button.setGraphic(null);
            button.setText((String) button.getProperties().remove(IDLE_TEXT));
            button.setAccessibleText((String) button.getProperties().remove(IDLE_ACCESSIBLE_TEXT));
            button.setDisable(false);
        }
    }
}
