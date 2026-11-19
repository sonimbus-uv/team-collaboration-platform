package mx.uv.sonimbus.desktop.ui.auth.recovery;

import javafx.animation.KeyFrame;
import javafx.animation.Timeline;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.control.TextFormatter;
import javafx.scene.control.ToggleButton;
import javafx.scene.layout.HBox;
import javafx.scene.layout.VBox;

import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.password.PasswordError;
import mx.uv.sonimbus.desktop.domain.password.PasswordException;
import mx.uv.sonimbus.desktop.domain.password.PasswordService;
import mx.uv.sonimbus.desktop.domain.password.RecoveryAttempt;
import mx.uv.sonimbus.desktop.domain.password.ResetPasswordRequest;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationValidator;
import mx.uv.sonimbus.desktop.ui.auth.common.AuthMessages;
import mx.uv.sonimbus.desktop.ui.auth.common.PasswordMessages;
import mx.uv.sonimbus.desktop.ui.auth.login.LoginNotice;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.common.PasswordVisibility;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

import java.time.Duration;
import java.time.Instant;
import java.util.Objects;

public final class RecoverPasswordController {
    private final PasswordService passwords;
    private final BackgroundTasks tasks;
    private final ViewManager views;
    private RecoveryAttempt attempt;
    private String recoveryEmail;
    private Instant resendAvailableAt = Instant.MIN;
    private Timeline timer;
    private boolean busy;
    private boolean active = true;
    private boolean resetStep;

    @FXML private HBox stepIndicator;
    @FXML private Label step1Badge;
    @FXML private Label step1Label;
    @FXML private Label step2Badge;
    @FXML private Label step2Label;
    @FXML private VBox emailStepBox;
    @FXML private TextField emailField;
    @FXML private Button sendCodeButton;
    @FXML private HBox sentInfoBox;
    @FXML private Label sentInfoIconLabel;
    @FXML private Label sentInfoLabel;
    @FXML private Hyperlink haveCodeLink;
    @FXML private VBox resetStepBox;
    @FXML private HBox resetErrorBox;
    @FXML private Label resetErrorLabel;
    @FXML private Label recoveryStatusLabel;
    @FXML private TextField codeField;
    @FXML private Hyperlink resendCodeLink;
    @FXML private PasswordField newPasswordField;
    @FXML private TextField newPasswordVisibleField;
    @FXML private ToggleButton showPasswordToggle;
    @FXML private PasswordField confirmPasswordField;
    @FXML private TextField confirmPasswordVisibleField;
    @FXML private ToggleButton showConfirmPasswordToggle;
    @FXML private HBox confirmPasswordErrorBox;
    @FXML private Label confirmPasswordErrorLabel;
    @FXML private Button resetButton;

    public RecoverPasswordController(
            PasswordService passwords, BackgroundTasks tasks, ViewManager views) {
        this.passwords = Objects.requireNonNull(passwords);
        this.tasks = Objects.requireNonNull(tasks);
        this.views = Objects.requireNonNull(views);
    }

    @FXML
    private void initialize() {
        PasswordVisibility.bind(newPasswordField, newPasswordVisibleField);
        PasswordVisibility.bind(confirmPasswordField, confirmPasswordVisibleField);
        codeField.setTextFormatter(
                new TextFormatter<String>(
                        change ->
                                change.getControlNewText().matches("[0-9]{0,8}") ? change : null));
        emailField
                .textProperty()
                .addListener(
                        (observable, previous, current) -> {
                            emailField.getStyleClass().remove("field-invalid");
                            AlertBox.hide(sentInfoBox);
                        });
        confirmPasswordField
                .textProperty()
                .addListener(
                        (observable, previous, current) -> AlertBox.hide(confirmPasswordErrorBox));
        resetButton
                .sceneProperty()
                .addListener(
                        (observable, previous, current) -> {
                            if (previous != null && current == null) {
                                active = false;
                                timer.stop();
                                clearSecrets();
                                attempt = null;
                            }
                        });
        timer =
                new Timeline(
                        new KeyFrame(
                                javafx.util.Duration.seconds(1), event -> updateAvailability()));
        timer.setCycleCount(Timeline.INDEFINITE);
        timer.play();
        updateAvailability();
    }

    @FXML
    private void onSendCode() {
        if (busy || !active) {
            return;
        }
        String email =
                RegistrationValidator.normalizeEmail(
                        emailField.getText() == null ? "" : emailField.getText());
        if (!RegistrationValidator.isValidEmailFormat(email)) {
            if (!emailField.getStyleClass().contains("field-invalid")) {
                emailField.getStyleClass().add("field-invalid");
            }
            AlertBox.show(
                    sentInfoBox,
                    sentInfoIconLabel,
                    sentInfoLabel,
                    AlertBox.Kind.ERROR,
                    "Escribe un correo electrónico válido.");
            emailField.requestFocus();
            return;
        }
        requestCode(email);
    }

    private void requestCode(String email) {
        setBusy(true);
        tasks.run(
                () -> passwords.forgotPassword(email),
                accepted -> {
                    if (!active) {
                        return;
                    }
                    attempt = accepted;
                    recoveryEmail = email;
                    codeField.clear();
                    clearSecrets();
                    resendAvailableAt = Instant.now().plusSeconds(60);
                    setBusy(false);
                    onShowResetStep();
                },
                this::onRequestFailed);
    }

    private void onRequestFailed(Throwable failure) {
        if (!active) {
            return;
        }
        if (failure instanceof PasswordException passwordFailure
                && (passwordFailure.kind() == PasswordError.UNCERTAIN
                        || passwordFailure.kind() == PasswordError.NETWORK
                        || passwordFailure.kind() == PasswordError.TIMEOUT
                        || passwordFailure.kind() == PasswordError.UNEXPECTED)) {
            attempt = null;
            resendAvailableAt = Instant.now().plusSeconds(60);
        }
        setBusy(false);
        if (failure instanceof PasswordException passwordFailure
                && passwordFailure.kind() == PasswordError.RATE_LIMITED) {
            long wait =
                    passwordFailure
                            .retryAfter()
                            .map(Duration::toSeconds)
                            .filter(seconds -> seconds > 0)
                            .orElse(60L);
            resendAvailableAt = Instant.now().plusSeconds(Math.min(wait, Integer.MAX_VALUE));
            updateAvailability();
        }
        showFailure(failure, true);
    }

    @FXML
    private void onShowResetStep() {
        if (busy || attempt == null || !active) {
            return;
        }
        resetStep = true;
        AlertBox.setShown(emailStepBox, false);
        AlertBox.setShown(resetStepBox, true);
        stepIndicator.setAccessibleText("Paso 2 de 2");
        step1Badge.getStyleClass().remove("step-active");
        step1Label.getStyleClass().remove("step-active");
        if (!step2Badge.getStyleClass().contains("step-active")) {
            step2Badge.getStyleClass().add("step-active");
            step2Label.getStyleClass().add("step-active");
        }
        sendCodeButton.setDefaultButton(false);
        resetButton.setDefaultButton(true);
        AlertBox.hide(resetErrorBox);
        updateAvailability();
        codeField.requestFocus();
    }

    @FXML
    private void onResendCode() {
        if (busy || recoveryEmail == null || Instant.now().isBefore(resendAvailableAt)) {
            return;
        }
        requestCode(recoveryEmail);
    }

    @FXML
    private void onTogglePasswordVisibility() {
        PasswordVisibility.apply(showPasswordToggle, newPasswordField, newPasswordVisibleField);
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
    private void onResetPassword() {
        if (busy || attempt == null || !Instant.now().isBefore(attempt.expiresAt())) {
            return;
        }
        ResetPasswordRequest request =
                new ResetPasswordRequest(
                        attempt.attemptId(),
                        codeField.getText(),
                        newPasswordField.getText(),
                        confirmPasswordField.getText());
        setBusy(true);
        tasks.run(
                () -> {
                    passwords.resetPassword(request);
                    return null;
                },
                ignored -> {
                    if (active) {
                        views.show(View.LOGIN, LoginNotice.PASSWORD_UPDATED);
                    }
                },
                this::onResetFailed);
    }

    private void onResetFailed(Throwable failure) {
        if (!active) {
            return;
        }
        clearSecrets();
        if (failure instanceof PasswordException passwordFailure
                && passwordFailure.kind() == PasswordError.UNCERTAIN) {
            attempt = null;
        }
        setBusy(false);
        showFailure(failure, false);
    }

    private void showFailure(Throwable failure, boolean requestingCode) {
        String message = AuthMessages.UNEXPECTED;
        if (failure instanceof PasswordException passwordFailure) {
            message =
                    requestingCode
                            ? PasswordMessages.recoveryFailure(passwordFailure)
                            : PasswordMessages.failure(passwordFailure);
            String confirmation = AuthMessages.forConfirmation(passwordFailure.problems());
            if (confirmation != null) {
                confirmPasswordErrorLabel.setText(confirmation);
                AlertBox.setShown(confirmPasswordErrorBox, true);
            }
        } else {
            AuthLog.unexpectedFailure("password-recovery", failure);
        }
        if (resetStep) {
            resetErrorLabel.setText(message);
            AlertBox.setShown(resetErrorBox, true);
        } else {
            AlertBox.show(
                    sentInfoBox, sentInfoIconLabel, sentInfoLabel, AlertBox.Kind.ERROR, message);
        }
    }

    private void setBusy(boolean value) {
        busy = value;
        BusyButton.setBusy(sendCodeButton, value && !resetStep, "Solicitando código…");
        BusyButton.setBusy(resetButton, value && resetStep, "Procesando…");
        AlertBox.hide(resetErrorBox);
        AlertBox.hide(sentInfoBox);
        AlertBox.hide(confirmPasswordErrorBox);
        updateAvailability();
    }

    private void updateAvailability() {
        long wait =
                Math.max(0, Duration.between(Instant.now(), resendAvailableAt).getSeconds() + 1);
        boolean usable = attempt != null && Instant.now().isBefore(attempt.expiresAt());
        emailField.setEditable(!busy);
        sendCodeButton.setDisable(busy || wait > 0);
        haveCodeLink.setDisable(busy || attempt == null);
        codeField.setDisable(busy || !usable);
        newPasswordField.setDisable(busy || !usable);
        newPasswordVisibleField.setDisable(busy || !usable);
        confirmPasswordField.setDisable(busy || !usable);
        confirmPasswordVisibleField.setDisable(busy || !usable);
        showConfirmPasswordToggle.setDisable(busy || !usable);
        showPasswordToggle.setDisable(busy || !usable);
        resetButton.setDisable(busy || !usable);
        resendCodeLink.setDisable(busy || wait > 0 || recoveryEmail == null);
        resendCodeLink.setText(
                wait > 0 ? "Solicitar otro código en " + wait + " s" : "Solicitar otro código");
        recoveryStatusLabel.setText(
                usable
                        ? PasswordMessages.RECOVERY_ACCEPTED
                        : "Solicita un nuevo código para continuar. Conserva abierta esta"
                                + " ventana.");
    }

    private void clearSecrets() {
        newPasswordField.clear();
        confirmPasswordField.clear();
    }

    @FXML
    private void onBackToLogin() {
        views.show(View.LOGIN);
    }
}
