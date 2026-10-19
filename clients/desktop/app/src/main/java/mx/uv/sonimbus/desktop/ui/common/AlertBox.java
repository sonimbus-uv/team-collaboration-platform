package mx.uv.sonimbus.desktop.ui.common;

import java.util.List;
import javafx.scene.Node;
import javafx.scene.control.Label;
import javafx.scene.layout.HBox;

/**
 * Muestra un aviso en las cajas {@code alert-*} de los FXML ({@code errorBox}, {@code infoBox}).
 * Cada tipo cambia color, icono y texto accesible a la vez: el estado nunca se distingue solo
 * por el color (RNF-USA-02).
 */
public final class AlertBox {

    public enum Kind {
        ERROR("alert-error", "alert-icon", "alert-text", "!", "Error"),
        INFO("alert-info", "info-icon", "info-text", "i", "Información"),
        SUCCESS("alert-success", "success-icon", "success-text", "✓", "Listo");

        private final String boxClass;
        private final String iconClass;
        private final String textClass;
        private final String icon;
        private final String accessibleName;

        Kind(String boxClass, String iconClass, String textClass, String icon, String accessibleName) {
            this.boxClass = boxClass;
            this.iconClass = iconClass;
            this.textClass = textClass;
            this.icon = icon;
            this.accessibleName = accessibleName;
        }
    }

    private static final List<Kind> KINDS = List.of(Kind.values());

    private AlertBox() {
    }

    public static void show(HBox box, Label icon, Label text, Kind kind, String message) {
        for (Kind other : KINDS) {
            box.getStyleClass().remove(other.boxClass);
            icon.getStyleClass().remove(other.iconClass);
            text.getStyleClass().remove(other.textClass);
        }
        box.getStyleClass().add(kind.boxClass);
        icon.getStyleClass().add(kind.iconClass);
        text.getStyleClass().add(kind.textClass);
        icon.setText(kind.icon);
        icon.setAccessibleText(kind.accessibleName);
        text.setText(message);
        box.setAccessibleText(kind.accessibleName + ": " + message);
        setShown(box, true);
    }

    public static void hide(HBox box) {
        setShown(box, false);
    }

    /** {@code visible} y {@code managed} juntos, para que el nodo oculto no deje hueco. */
    public static void setShown(Node node, boolean shown) {
        node.setVisible(shown);
        node.setManaged(shown);
    }
}
