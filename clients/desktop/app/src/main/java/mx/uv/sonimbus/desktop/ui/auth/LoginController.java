package mx.uv.sonimbus.desktop.ui.auth;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.control.ToggleButton;
import javafx.scene.layout.HBox;
import mx.uv.sonimbus.desktop.ui.navigation.Navigable;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

public class LoginController implements Navigable {

    private ViewManager viewManager;

    @FXML private TextField emailField;
    @FXML private PasswordField passwordField;
    @FXML private TextField passwordVisibleField;
    @FXML private ToggleButton showPasswordToggle;
    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private Button loginButton;
    @FXML private Button googleButton;
    @FXML private Hyperlink forgotPasswordLink;
    @FXML private Hyperlink createAccountLink;

    @FXML
    private void initialize() {
        // TODO CU-03: enlazar passwordVisibleField.textProperty() con passwordField.textProperty()
        // TODO CU-03: limpiar errorBox al editar emailField o passwordField
    }

    @FXML
    private void onLogin() {
        // TODO CU-03: validar campos, autenticar y mostrar errorBox con mensaje genérico si falla
    }

    @FXML
    private void onTogglePasswordVisibility() {
        // TODO CU-03: alternar visible/managed entre passwordField y passwordVisibleField
        //             y cambiar texto a "Ocultar"/"Mostrar"
    }

    @FXML
    private void onGoogleLogin() {
        // TODO CU-04: iniciar flujo OAuth con Google
    }

    @FXML
    private void onForgotPassword() {
        viewManager.show(View.RECOVER_PASSWORD);
    }

    @FXML
    private void onCreateAccount() {
        viewManager.show(View.REGISTER);
    }

    @Override
    public void setViewManager(ViewManager viewManager) {
        this.viewManager = viewManager;
    }
}
