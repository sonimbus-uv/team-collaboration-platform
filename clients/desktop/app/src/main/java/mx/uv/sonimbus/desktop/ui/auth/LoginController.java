package mx.uv.sonimbus.desktop.ui.auth;

import javafx.fxml.FXML;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;

/** CU-03 Iniciar sesión. Por ahora solo maqueta; se conecta a Auth en otra rama. */
public class LoginController {

    @FXML private TextField emailField;
    @FXML private PasswordField passwordField;
    @FXML private Label errorLabel;

    @FXML
    private void initialize() {
        errorLabel.setText("");
    }

    @FXML
    private void onLogin() {
        errorLabel.setText("Pendiente: conectar con Auth Service");
    }
}
