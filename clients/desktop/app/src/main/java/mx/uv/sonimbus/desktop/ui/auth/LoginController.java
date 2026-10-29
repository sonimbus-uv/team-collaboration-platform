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
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.AuthApi;
import mx.uv.sonimbus.desktop.data.session.Session;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.common.PasswordVisibility;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

/**
 * CU-03. Inicia la sesión de escritorio con Auth y abre el shell. La sesión vive solo en memoria
 * ({@link Session}).
 */
public class LoginController {

    private static final String FIELD_INVALID = "field-invalid";

    private final ViewManager viewManager;
    private final AuthApi authApi;
    private final Session session;
    private final BackgroundTasks tasks;

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

    public LoginController(ViewManager viewManager, AuthApi authApi, Session session, BackgroundTasks tasks) {
        this.viewManager = Objects.requireNonNull(viewManager);
        this.authApi = Objects.requireNonNull(authApi);
        this.session = Objects.requireNonNull(session);
        this.tasks = Objects.requireNonNull(tasks);
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
        String email = emailField.getText().strip();
        String password = passwordField.getText();
        BusyButton.setBusy(loginButton, true, "Iniciando sesión…");
        tasks.run(() -> authApi.login(email, password), tokens -> {
            session.start(tokens);
            viewManager.show(View.MAIN);
        }, error -> {
            BusyButton.setBusy(loginButton, false, null);
            String message = error instanceof ApiException e ? AuthMessages.forLogin(e) : AuthMessages.UNEXPECTED;
            AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.ERROR, message);
            passwordField.clear();
            visiblePasswordField().requestFocus();
        });
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
