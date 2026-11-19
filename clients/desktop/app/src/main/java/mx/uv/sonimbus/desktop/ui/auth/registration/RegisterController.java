package mx.uv.sonimbus.desktop.ui.auth.registration;

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
import mx.uv.sonimbus.desktop.domain.registration.PendingRegistration;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationError;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationException;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationRequest;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationService;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator.Problem;
import mx.uv.sonimbus.desktop.ui.auth.common.AuthMessages;
import mx.uv.sonimbus.desktop.ui.auth.common.GoogleMessages;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.common.PasswordVisibility;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

import java.util.Objects;
import java.util.Set;
import java.util.stream.Collectors;
import java.util.stream.Stream;

public class RegisterController {

    private static final String FIELD_INVALID = "field-invalid";

    private final RegistrationService registrations;
    private final BackgroundTasks backgroundTasks;
    private final ViewManager viewManager;

    private final GoogleService google;
    private boolean googlePending;
    @FXML private Button cancelGoogleButton;
    private boolean submitting;
    private boolean active = true;

    @FXML private TextField nameField;
    @FXML private TextField emailField;

    @FXML private PasswordField passwordField;
    @FXML private TextField passwordVisibleField;
    @FXML private ToggleButton showPasswordToggle;
    @FXML private Label passwordHintLabel;

    @FXML private PasswordField confirmPasswordField;
    @FXML private TextField confirmPasswordVisibleField;
    @FXML private ToggleButton showConfirmPasswordToggle;
    @FXML private HBox confirmPasswordErrorBox;
    @FXML private Label confirmPasswordErrorLabel;

    @FXML private HBox errorBox;
    @FXML private Label errorIconLabel;
    @FXML private Label errorLabel;

    @FXML private Button registerButton;
    @FXML private Button googleButton;
    @FXML private Hyperlink backToLoginLink;

    public RegisterController(
            RegistrationService registrations,
            BackgroundTasks backgroundTasks,
            ViewManager viewManager,
            GoogleService google) {
        this.registrations = Objects.requireNonNull(registrations);
        this.backgroundTasks = Objects.requireNonNull(backgroundTasks);
        this.viewManager = Objects.requireNonNull(viewManager);
        this.google = Objects.requireNonNull(google);
    }

    @FXML
    private void initialize() {
        PasswordVisibility.bind(passwordField, passwordVisibleField);
        PasswordVisibility.bind(confirmPasswordField, confirmPasswordVisibleField);

        registerButton
                .sceneProperty()
                .addListener(
                        (observable, previous, current) -> {
                            if (previous != null && current == null) {
                                leave();
                            }
                        });

        passwordHintLabel.setText(AuthMessages.PASSWORD_RULE);

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

        confirmPasswordField
                .textProperty()
                .addListener(
                        (observable, previousText, currentText) -> {
                            clearError(confirmPasswordField);
                            clearError(confirmPasswordVisibleField);
                        });
    }

    @FXML
    private void onRegister() {

        if (submitting) {
            return;
        }

        RegistrationRequest request =
                new RegistrationRequest(
                        emailField.getText(),
                        passwordField.getText(),
                        confirmPasswordField.getText());
        RegistrationValidator.Result validation = registrations.validate(request);

        if (!validation.isValid()) {
            showValidationProblems(validation.problems());
            return;
        }

        hideMessages();
        setSubmitting(true);

        String email = validation.email();

        backgroundTasks.run(
                () -> registrations.register(request),
                attempt -> onRegistered(email, attempt),
                this::onRegisterFailed);
    }

    private void onRegistered(String email, PendingRegistration attempt) {

        if (!active) {
            return;
        }

        setSubmitting(false);
        passwordField.clear();
        confirmPasswordField.clear();

        leave();
        viewManager.show(
                View.VERIFY_ACCOUNT, VerifyAccountParams.afterRegistration(email, attempt));
    }

    private void onRegisterFailed(Throwable error) {

        if (!active) {
            return;
        }

        setSubmitting(false);

        if (!(error instanceof RegistrationException failure)) {
            AuthLog.unexpectedFailure("register", error);
            showAlert(AlertBox.Kind.ERROR, AuthMessages.UNEXPECTED);
            return;
        }

        if (!failure.problems().isEmpty()) {
            showValidationProblems(failure.problems());
            return;
        }

        showAlert(AlertBox.Kind.ERROR, AuthMessages.forRegister(failure));

        if (failure.kind() == RegistrationError.EMAIL_TAKEN) {
            markInvalid(emailField);
        }
    }

    @FXML
    private void onTogglePasswordVisibility() {
        PasswordVisibility.apply(showPasswordToggle, passwordField, passwordVisibleField);
    }

    @FXML
    private void onToggleConfirmPasswordVisibility() {
        PasswordVisibility.apply(
                showConfirmPasswordToggle, confirmPasswordField, confirmPasswordVisibleField);
        showConfirmPasswordToggle.setAccessibleText(
                showConfirmPasswordToggle.isSelected()
                        ? "Ocultar confirmación de contraseña"
                        : "Mostrar confirmación de contraseña");
    }

    @FXML
    private void onGoogleRegister() {
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
    private void onBackToLogin() {
        leave();
        viewManager.show(View.LOGIN);
    }

    private void showValidationProblems(Set<Problem> problems) {

        hideMessages();

        for (TextInputControl field :
                new TextInputControl[] {
                    emailField,
                    passwordField,
                    passwordVisibleField,
                    confirmPasswordField,
                    confirmPasswordVisibleField
                }) {
            field.getStyleClass().remove(FIELD_INVALID);
        }

        String message =
                Stream.of(AuthMessages.forEmail(problems), AuthMessages.forPassword(problems))
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
            markInvalid(confirmPasswordVisibleField);
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
            (showConfirmPasswordToggle.isSelected()
                            ? confirmPasswordVisibleField
                            : confirmPasswordField)
                    .requestFocus();
        }
    }

    private void showAlert(AlertBox.Kind kind, String message) {
        AlertBox.show(errorBox, errorIconLabel, errorLabel, kind, message);
    }

    private void hideMessages() {
        AlertBox.hide(errorBox);
        AlertBox.hide(confirmPasswordErrorBox);
    }

    private void setSubmitting(boolean requestInProgress) {
        submitting = requestInProgress;
        BusyButton.setBusy(registerButton, requestInProgress, "Creando cuenta…");

        for (TextInputControl field :
                new TextInputControl[] {
                    emailField,
                    passwordField,
                    passwordVisibleField,
                    confirmPasswordField,
                    confirmPasswordVisibleField
                }) {
            field.setEditable(!requestInProgress);
        }

        googleButton.setDisable(requestInProgress);
        showPasswordToggle.setDisable(requestInProgress);
        showConfirmPasswordToggle.setDisable(requestInProgress);
    }

    private void markInvalid(TextInputControl field) {
        if (!field.getStyleClass().contains(FIELD_INVALID)) {
            field.getStyleClass().add(FIELD_INVALID);
        }
    }

    private void clearError(TextInputControl field) {

        if (submitting) {
            return;
        }

        field.getStyleClass().remove(FIELD_INVALID);

        if (field == confirmPasswordField || field == confirmPasswordVisibleField) {
            AlertBox.hide(confirmPasswordErrorBox);
        } else {
            AlertBox.hide(errorBox);
        }
    }

    private void leave() {
        active = false;
        if (googlePending) {
            google.cancel();
        }
    }
}
