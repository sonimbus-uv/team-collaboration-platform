package mx.uv.sonimbus.desktop.ui.auth;

import java.util.Objects;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.control.TextInputControl;
import javafx.scene.control.ToggleButton;
import javafx.scene.layout.HBox;
import mx.uv.sonimbus.desktop.data.api.Account;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.AuthApi;
import mx.uv.sonimbus.desktop.domain.session.RegistrationValidator;
import mx.uv.sonimbus.desktop.domain.session.RegistrationValidator.Problem;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.common.PasswordVisibility;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

/** CU-01: registro con correo y contraseña. */
public class RegisterController {

    private static final String FIELD_INVALID = "field-invalid";

    private final AuthApi authApi;
    private final BackgroundTasks tasks;
    private final ViewManager viewManager;

    /** Evita dobles envíos (Enter repetido o doble clic) mientras la petición está en curso. */
    private boolean submitting;
    /** Falso al salir de la vista: una respuesta tardía ya no debe navegar ni tocar la interfaz. */
    private boolean active = true;
    /** Correo normalizado del último 409, para el enlace de reenvío. */
    private String takenEmail;

    @FXML private TextField nameField;      // Oculto hasta resolver #14 (Auth no recibe el nombre)
    @FXML private TextField emailField;
    @FXML private PasswordField passwordField;
    @FXML private TextField passwordVisibleField;
    @FXML private ToggleButton showPasswordToggle;
    @FXML private Label passwordHintLabel;
    @FXML private PasswordField confirmPasswordField;
    @FXML private HBox confirmPasswordErrorBox;
    @FXML private Label confirmPasswordErrorLabel;
    @FXML private HBox errorBox;
    @FXML private Label errorIconLabel;
    @FXML private Label errorLabel;
    @FXML private Hyperlink resendVerificationLink;
    @FXML private Button registerButton;
    @FXML private Button googleButton;
    @FXML private Hyperlink backToLoginLink;

    public RegisterController(AuthApi authApi, BackgroundTasks tasks, ViewManager viewManager) {
        this.authApi = Objects.requireNonNull(authApi);
        this.tasks = Objects.requireNonNull(tasks);
        this.viewManager = Objects.requireNonNull(viewManager);
    }

    @FXML
    private void initialize() {
        PasswordVisibility.bind(passwordField, passwordVisibleField);
        passwordHintLabel.setText(AuthMessages.PASSWORD_RULE);
        emailField.textProperty().addListener((obs, old, text) -> clearError(emailField));
        passwordField.textProperty().addListener((obs, old, text) -> {
            clearError(passwordField);
            clearError(passwordVisibleField);
        });
        confirmPasswordField.textProperty().addListener((obs, old, text) -> clearError(confirmPasswordField));
    }

    @FXML
    private void onRegister() {
        if (submitting) {
            return;
        }
        RegistrationValidator.Result result = RegistrationValidator.validate(
                emailField.getText(), passwordField.getText(), confirmPasswordField.getText());
        if (!result.isValid()) {
            showValidationProblems(result.problems());
            return;
        }
        hideMessages();
        setSubmitting(true);
        String email = result.email();
        String password = passwordField.getText();
        tasks.run(() -> authApi.register(email, password),
                account -> onRegistered(email, account),
                this::onRegisterFailed);
    }

    private void onRegistered(String email, Account account) {
        if (!active) {
            return;
        }
        setSubmitting(false);
        String sentTo = account.email() == null || account.email().isBlank() ? email : account.email();
        leave();
        viewManager.show(View.VERIFY_ACCOUNT, VerifyAccountParams.afterRegistration(sentTo, account.verificationEmailSent()));
    }

    private void onRegisterFailed(Throwable error) {
        if (!active) {
            return;
        }
        setSubmitting(false);
        if (!(error instanceof ApiException e)) {
            showAlert(AlertBox.Kind.ERROR, AuthMessages.UNEXPECTED);
            return;
        }
        showAlert(AlertBox.Kind.ERROR, AuthMessages.forRegister(e));
        if (e.kind() == ApiErrorKind.EMAIL_TAKEN) {
            takenEmail = RegistrationValidator.normalizeEmail(emailField.getText());
            markInvalid(emailField);
            AlertBox.setShown(resendVerificationLink, true);
        }
    }

    /** Desde el aviso del 409: la pantalla de verificación permite reenviar el enlace. */
    @FXML
    private void onResendVerification() {
        if (takenEmail == null) {
            return;
        }
        leave();
        viewManager.show(View.VERIFY_ACCOUNT, VerifyAccountParams.forExistingAccount(takenEmail));
    }

    @FXML
    private void onTogglePasswordVisibility() {
        PasswordVisibility.apply(showPasswordToggle, passwordField, passwordVisibleField);
    }

    @FXML
    private void onGoogleRegister() {
        // TODO CU-04: registro/inicio con Google (OAuth) cuando Auth tenga el endpoint
        showAlert(AlertBox.Kind.INFO, AuthMessages.GOOGLE_UNAVAILABLE);
    }

    @FXML
    private void onBackToLogin() {
        leave();
        viewManager.show(View.LOGIN);
    }

    private void showValidationProblems(Set<Problem> problems) {
        hideMessages();
        for (TextInputControl field : new TextInputControl[] {
                emailField, passwordField, passwordVisibleField, confirmPasswordField}) {
            field.getStyleClass().remove(FIELD_INVALID);
        }
        String message = Stream.of(AuthMessages.forEmail(problems), AuthMessages.forPassword(problems))
                .filter(Objects::nonNull)
                .collect(Collectors.joining("\n"));
        if (message.isEmpty()) {
            AlertBox.hide(errorBox);
        } else {
            showAlert(AlertBox.Kind.ERROR, message);
        }
        String confirmation = AuthMessages.forConfirmation(problems);
        if (confirmation != null) {
            confirmPasswordErrorLabel.setText(confirmation);
            markInvalid(confirmPasswordField);
            AlertBox.setShown(confirmPasswordErrorBox, true);
        }
        if (problems.stream().anyMatch(Problem::concernsEmail)) {
            markInvalid(emailField);
        }
        if (problems.stream().anyMatch(Problem::concernsPassword)) {
            markInvalid(passwordField);
            markInvalid(passwordVisibleField);
        }
        focusFirstInvalid(problems);
    }

    private void focusFirstInvalid(Set<Problem> problems) {
        if (problems.stream().anyMatch(Problem::concernsEmail)) {
            emailField.requestFocus();
        } else if (problems.stream().anyMatch(Problem::concernsPassword)) {
            (showPasswordToggle.isSelected() ? passwordVisibleField : passwordField).requestFocus();
        } else {
            confirmPasswordField.requestFocus();
        }
    }

    private void showAlert(AlertBox.Kind kind, String message) {
        AlertBox.setShown(resendVerificationLink, false);
        AlertBox.show(errorBox, errorIconLabel, errorLabel, kind, message);
    }

    private void hideMessages() {
        AlertBox.hide(errorBox);
        AlertBox.setShown(resendVerificationLink, false);
        AlertBox.hide(confirmPasswordErrorBox);
    }

    private void setSubmitting(boolean value) {
        submitting = value;
        BusyButton.setBusy(registerButton, value, "Creando cuenta…");
        for (TextInputControl field : new TextInputControl[] {
                emailField, passwordField, passwordVisibleField, confirmPasswordField}) {
            field.setEditable(!value);
        }
        googleButton.setDisable(value);
    }

    private void markInvalid(TextInputControl field) {
        if (!field.getStyleClass().contains(FIELD_INVALID)) {
            field.getStyleClass().add(FIELD_INVALID);
        }
    }

    /** Al editar un campo se quita su marca; los avisos generales desaparecen con él. */
    private void clearError(TextInputControl field) {
        if (submitting) {
            return;
        }
        field.getStyleClass().remove(FIELD_INVALID);
        if (field == confirmPasswordField) {
            AlertBox.hide(confirmPasswordErrorBox);
        } else {
            AlertBox.hide(errorBox);
        }
    }

    private void leave() {
        active = false;
    }
}
