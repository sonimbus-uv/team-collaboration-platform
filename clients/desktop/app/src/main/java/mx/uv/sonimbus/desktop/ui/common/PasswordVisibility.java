package mx.uv.sonimbus.desktop.ui.common;

import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.control.ToggleButton;

public final class PasswordVisibility {

    private PasswordVisibility() {
    }

    public static void bind(PasswordField hidden, TextField visible) {
        visible.textProperty().bindBidirectional(hidden.textProperty());
    }

    public static void apply(ToggleButton toggle, PasswordField hidden, TextField visible) {
        boolean show = toggle.isSelected();
        TextField from = show ? hidden : visible;
        TextField to = show ? visible : hidden;
        int caret = from.getCaretPosition();
        AlertBox.setShown(visible, show);
        AlertBox.setShown(hidden, !show);
        toggle.setText(show ? "Ocultar" : "Mostrar");
        toggle.setAccessibleText(show ? "Ocultar contraseña" : "Mostrar contraseña");
        to.requestFocus();
        to.positionCaret(caret);
    }
}
