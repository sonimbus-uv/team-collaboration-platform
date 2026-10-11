package mx.uv.sonimbus.desktop.ui.auth;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.control.ToggleButton;
import javafx.scene.layout.HBox;

public class RegisterController {

    @FXML private TextField nameField;
    @FXML private TextField emailField;
    @FXML private PasswordField passwordField;
    @FXML private TextField passwordVisibleField;
    @FXML private ToggleButton showPasswordToggle;
    @FXML private Label passwordHintLabel;
    @FXML private PasswordField confirmPasswordField;
    @FXML private HBox confirmPasswordErrorBox;
    @FXML private Label confirmPasswordErrorLabel;
    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private Button registerButton;
    @FXML private Button googleButton;
    @FXML private Hyperlink backToLoginLink;

    @FXML
    private void initialize() {
        // TODO CU-01: enlazar passwordVisibleField.textProperty() con passwordField.textProperty()
        // TODO CU-01: ocultar errores y quitar la clase "field-invalid" al editar los campos
    }

    @FXML
    private void onRegister() {
        // TODO CU-01: validar campos obligatorios, formato de correo, reglas de contraseña y
        //             coincidencia; si no coinciden, agregar "field-invalid" a confirmPasswordField
        //             y mostrar confirmPasswordErrorBox; si todo es válido, registrar y
        //             ViewManager.show(View.VERIFY_ACCOUNT) (CU-02)
    }

    @FXML
    private void onTogglePasswordVisibility() {
        // TODO CU-01: alternar visible/managed entre passwordField y passwordVisibleField
        //             y cambiar texto a "Ocultar"/"Mostrar"
    }

    @FXML
    private void onGoogleRegister() {
        // TODO CU-04: registro/inicio con Google (OAuth)
    }

    @FXML
    private void onBackToLogin() {
        // TODO CU-03: ViewManager.show(View.LOGIN)
    }
}
