package mx.uv.sonimbus.desktop.ui.common;

import java.util.Objects;
import java.util.Optional;
import javafx.event.ActionEvent;
import javafx.scene.Node;
import javafx.scene.control.Alert;
import javafx.scene.control.ButtonType;
import javafx.scene.control.Dialog;
import javafx.scene.control.Label;
import javafx.scene.control.TextField;
import javafx.scene.control.TextInputDialog;
import javafx.scene.input.Clipboard;
import javafx.scene.input.ClipboardContent;
import javafx.scene.layout.VBox;
import javafx.stage.Window;

public final class Dialogs {

    private static final String THEME = "/mx/uv/sonimbus/desktop/css/theme.css";

    private Dialogs() {
    }

    public static Optional<String> askText(Node owner, String title, String header, String initial) {
        TextInputDialog dialog = new TextInputDialog(initial);
        prepare(dialog, owner, title, header);
        return dialog.showAndWait().map(String::strip);
    }

    public static void showError(Node owner, String message) {
        Alert alert = new Alert(Alert.AlertType.ERROR, message, ButtonType.OK);
        prepare(alert, owner, "Error", null);
        alert.showAndWait();
    }

    public static void showInvitationCode(Node owner, String groupName, String code) {
        TextField codeField = new TextField(code);
        codeField.setEditable(false);
        codeField.getStyleClass().add("code-field-compact");
        codeField.setAccessibleText("Código de invitación");
        Label hint = new Label("Compártelo con quien quieras invitar. Caduca en 7 días.");
        hint.getStyleClass().add("hint-text");
        hint.setWrapText(true);

        ButtonType copy = new ButtonType("Copiar");
        Dialog<ButtonType> dialog = new Dialog<>();
        prepare(dialog, owner, "Invitar a " + groupName, "Código de invitación");
        dialog.getDialogPane().setContent(new VBox(8, codeField, hint));
        dialog.getDialogPane().getButtonTypes().setAll(copy, ButtonType.CLOSE);
        dialog.getDialogPane().lookupButton(copy).addEventFilter(ActionEvent.ACTION, event -> {
            ClipboardContent content = new ClipboardContent();
            content.putString(code);
            Clipboard.getSystemClipboard().setContent(content);
            event.consume();
        });
        dialog.showAndWait();
    }

    private static void prepare(Dialog<?> dialog, Node owner, String title, String header) {
        Window window = owner.getScene() == null ? null : owner.getScene().getWindow();
        if (window != null) {
            dialog.initOwner(window);
        }
        dialog.setTitle(title);
        dialog.setHeaderText(header);
        dialog.getDialogPane().getStylesheets().add(
                Objects.requireNonNull(Dialogs.class.getResource(THEME)).toExternalForm());
        dialog.getDialogPane().getStyleClass().add("dialog-root");
    }
}
