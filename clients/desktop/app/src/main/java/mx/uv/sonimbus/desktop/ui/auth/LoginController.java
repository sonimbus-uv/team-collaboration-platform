package mx.uv.sonimbus.desktop.ui.auth;

import java.util.Objects;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.control.TextInputControl;
import javafx.scene.control.ToggleButton;
import javafx.scene.layout.HBox;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.PasswordVisibility;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

/**
 * CU-03. Auth todavía no tiene inicio de sesión: la vista valida los campos y avisa que no está
 * disponible. {@code AuthApi} no tiene método de login a propósito, para no inventar el contrato.
 */
public class LoginController {

    private static final String FIELD_INVALID = "field-invalid";

    private final ViewManager viewManager;

    @FXML private TextField emailField;
    @FXML private PasswordField passwordField;
    @FXML private TextField passwordVisibleField;
    @FXML private ToggleButton showPasswordToggle;
    @FXML private HBox errorBox;
    @FXML private Label errorIconLabel;
    @FXML private Label errorLabel;
    @FXML private Button loginButton;
    @FXML private Button googleButton;
    @FXML private Hyperlink forgotPasswordLink;
    @FXML private Hyperlink createAccountLink;

    public LoginController(ViewManager viewManager) {
        this.viewManager = Objects.requireNonNull(viewManager);
    }

    @FXML
    private void initialize() {
        PasswordVisibility.bind(passwordField, passwordVisibleField);
        emailField.textProperty().addListener((obs, old, text) -> clearError(emailField));
        passwordField.textProperty().addListener((obs, old, text) -> {
            clearError(passwordField);
            clearError(passwordVisibleField);
        });
    }

    @FXML
    private void onLogin() {
        boolean emailEmpty = emailField.getText() == null || emailField.getText().isBlank();
        boolean passwordEmpty = passwordField.getText() == null || passwordField.getText().isEmpty();
        if (emailEmpty || passwordEmpty) {
            if (emailEmpty) {
                markInvalid(emailField);
            }
            if (passwordEmpty) {
                markInvalid(passwordField);
                markInvalid(passwordVisibleField);
            }
            AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.ERROR, AuthMessages.LOGIN_FIELDS_EMPTY);
            (emailEmpty ? emailField : visiblePasswordField()).requestFocus();
            return;
        }
        // TODO CU-03 (#35): autenticar con Auth cuando exista el endpoint; si falla, mensaje
        //                   genérico «Correo o contraseña incorrectos» (Fig. 10b) y, si funciona,
        //                   guardar la sesión de forma segura y viewManager.show(View.MAIN).
        AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.INFO, AuthMessages.LOGIN_UNAVAILABLE);
    }

    @FXML
    private void onTogglePasswordVisibility() {
        PasswordVisibility.apply(showPasswordToggle, passwordField, passwordVisibleField);
    }

    @FXML
    private void onGoogleLogin() {
        // TODO CU-04: iniciar flujo OAuth con Google cuando Auth tenga el endpoint
        AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.INFO, AuthMessages.GOOGLE_UNAVAILABLE);
    }

    @FXML
    private void onForgotPassword() {
        viewManager.show(View.RECOVER_PASSWORD);
    }

    @FXML
    private void onCreateAccount() {
        viewManager.show(View.REGISTER);
    }

    private TextField visiblePasswordField() {
        return showPasswordToggle.isSelected() ? passwordVisibleField : passwordField;
    }

    private void markInvalid(TextInputControl field) {
        if (!field.getStyleClass().contains(FIELD_INVALID)) {
            field.getStyleClass().add(FIELD_INVALID);
        }
    }

    private void clearError(TextInputControl field) {
        field.getStyleClass().remove(FIELD_INVALID);
        AlertBox.hide(errorBox);
    }
}
