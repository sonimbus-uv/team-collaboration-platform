package mx.uv.sonimbus.desktop.ui.common;

import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.control.ToggleButton;

/** «Mostrar / Ocultar» contraseña: alterna entre el {@code PasswordField} y un campo visible. */
public final class PasswordVisibility {

    private PasswordVisibility() {
    }

    /** Enlaza los textos de ambos campos; se llama una vez en {@code initialize()}. */
    public static void bind(PasswordField hidden, TextField visible) {
        visible.textProperty().bindBidirectional(hidden.textProperty());
    }

    /** Muestra el campo que corresponde al estado del botón y conserva el foco y el cursor. */
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
