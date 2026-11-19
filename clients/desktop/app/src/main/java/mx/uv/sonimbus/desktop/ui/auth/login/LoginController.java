package mx.uv.sonimbus.desktop.ui.auth.login;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.control.TextInputControl;
import javafx.scene.control.ToggleButton;
import javafx.scene.layout.HBox;

import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.google.GoogleException;
import mx.uv.sonimbus.desktop.domain.google.GoogleService;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.domain.session.LoginCredentials;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.ui.auth.common.AuthMessages;
import mx.uv.sonimbus.desktop.ui.auth.common.GoogleMessages;
import mx.uv.sonimbus.desktop.ui.auth.common.PasswordMessages;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.common.PasswordVisibility;
import mx.uv.sonimbus.desktop.ui.navigation.Parameterized;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

import java.util.Objects;

public class LoginController implements Parameterized<LoginNotice> {

    private static final String FIELD_INVALID = "field-invalid";

    private final ViewManager viewManager;
    private final SessionService sessions;
    private final BackgroundTasks backgroundTasks;
    private final GoogleService google;
    private boolean googlePending;
    @FXML private Button cancelGoogleButton;
    private boolean submitting;
    private boolean active = true;

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

    public LoginController(
            SessionService sessions,
            BackgroundTasks backgroundTasks,
            ViewManager viewManager,
            GoogleService google) {
        this.viewManager = Objects.requireNonNull(viewManager);
        this.google = Objects.requireNonNull(google);
        this.sessions = Objects.requireNonNull(sessions);
        this.backgroundTasks = Objects.requireNonNull(backgroundTasks);
    }

    @Override
    public Class<LoginNotice> parametersType() {
        return LoginNotice.class;
    }

    @Override
    public void setParameters(LoginNotice notice) {
        String message =
                switch (notice) {
                    case PASSWORD_UPDATED -> PasswordMessages.UPDATED;
                    case PASSWORD_UNCERTAIN -> PasswordMessages.UNCERTAIN;
                    case SESSION_EXPIRED -> "La sesión terminó. Inicia sesión de nuevo.";
                };
        AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.INFO, message);
    }

    @FXML
    private void initialize() {
        PasswordVisibility.bind(passwordField, passwordVisibleField);
        loginButton
                .sceneProperty()
                .addListener(
                        (observable, previous, current) -> {
                            if (previous != null && current == null) {
                                active = false;
                                if (googlePending) {
                                    google.cancel();
                                }
                                passwordField.clear();
                            }
                        });
        emailField
                .textProperty()
                .addListener((observable, previousText, currentText) -> clearError(emailField));
        passwordField
                .textProperty()
                .addListener(
                        (observable, previousText, currentText) -> {
                            clearError(passwordField);
                            clearError(passwordVisibleField);
                        });
    }

    @FXML
    private void onLogin() {
        if (submitting || !active) {
            return;
        }
        boolean emailEmpty = emailField.getText() == null || emailField.getText().isBlank();
        boolean passwordEmpty =
                passwordField.getText() == null || passwordField.getText().isEmpty();
        if (emailEmpty || passwordEmpty) {
            if (emailEmpty) {
                markInvalid(emailField);
            }
            if (passwordEmpty) {
                markInvalid(passwordField);
                markInvalid(passwordVisibleField);
            }
            AlertBox.show(
                    errorBox,
                    errorIconLabel,
                    errorLabel,
                    AlertBox.Kind.ERROR,
                    AuthMessages.LOGIN_FIELDS_EMPTY);
            (emailEmpty ? emailField : visiblePasswordField()).requestFocus();
            return;
        }

        LoginCredentials credentials =
                new LoginCredentials(emailField.getText(), passwordField.getText());
        AlertBox.hide(errorBox);
        setSubmitting(true);
        backgroundTasks.run(
                () -> {
                    sessions.login(credentials);
                    return null;
                },
                ignored -> onAuthenticated(),
                this::onLoginFailed);
    }

    private void onAuthenticated() {
        if (!active) {
            return;
        }
        passwordField.clear();
        setSubmitting(false);
        viewManager.show(View.MAIN);
    }

    private void onLoginFailed(Throwable failure) {
        if (!active) {
            return;
        }
        setSubmitting(false);
        passwordField.clear();
        String message = AuthMessages.UNEXPECTED;
        if (failure instanceof AuthenticationException authenticationFailure) {
            message = AuthMessages.forLogin(authenticationFailure);
        } else {
            AuthLog.unexpectedFailure("login", failure);
        }
        AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.ERROR, message);
        visiblePasswordField().requestFocus();
    }

    private void setSubmitting(boolean busy) {
        submitting = busy;
        BusyButton.setBusy(loginButton, busy, "Iniciando sesión…");
        emailField.setEditable(!busy);
        passwordField.setEditable(!busy);
        passwordVisibleField.setEditable(!busy);
        showPasswordToggle.setDisable(busy);
        googleButton.setDisable(busy);
        forgotPasswordLink.setDisable(busy);
        createAccountLink.setDisable(busy);
    }

    @FXML
    private void onTogglePasswordVisibility() {
        PasswordVisibility.apply(showPasswordToggle, passwordField, passwordVisibleField);
    }

    @FXML
    private void onGoogleLogin() {
        if (submitting || !active) {
            return;
        }
        googlePending = true;
        setSubmitting(true);
        AlertBox.hide(errorBox);
        AlertBox.setShown(cancelGoogleButton, true);
        BusyButton.setBusy(googleButton, true, "Esperando a Google…");
        backgroundTasks.run(
                () -> {
                    google.signIn();
                    return null;
                },
                ignored -> {
                    if (active) {
                        googlePending = false;
                        viewManager.show(View.MAIN);
                        viewManager.bringToFront();
                    }
                },
                this::onGoogleFailed);
    }

    private void onGoogleFailed(Throwable failure) {
        if (!active) {
            return;
        }
        googlePending = false;
        BusyButton.setBusy(googleButton, false, null);
        AlertBox.setShown(cancelGoogleButton, false);
        setSubmitting(false);
        String message = AuthMessages.UNEXPECTED;
        if (failure instanceof GoogleException googleFailure) {
            message = GoogleMessages.failure(googleFailure);
        } else {
            AuthLog.unexpectedFailure("google-login", failure);
        }
        AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.ERROR, message);
        viewManager.bringToFront();
    }

    @FXML
    private void onCancelGoogle() {
        google.cancel();
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
