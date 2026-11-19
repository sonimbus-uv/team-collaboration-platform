package mx.uv.sonimbus.desktop.ui.auth.registration;

import javafx.animation.KeyFrame;
import javafx.animation.Timeline;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.TextField;
import javafx.scene.control.TextFormatter;
import javafx.scene.layout.HBox;
import javafx.scene.text.Text;

import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.registration.PendingRegistration;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationError;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationException;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationService;
import mx.uv.sonimbus.desktop.ui.auth.common.AuthMessages;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.navigation.Parameterized;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

import java.time.Duration;
import java.time.Instant;
import java.util.Objects;

public class VerifyAccountController implements Parameterized<VerifyAccountParams> {

    static final int RESEND_COOLDOWN_SECONDS = 60;
    private static final int CODE_LENGTH = 8;
    private final RegistrationService registrations;
    private final BackgroundTasks backgroundTasks;
    private final ViewManager viewManager;

    private VerifyAccountParams parameters;
    private Instant resendAvailableAt = Instant.MIN;
    private Timeline timer;
    private boolean active = true;
    private boolean busy;
    private boolean verified;

    @FXML private Text sentToEmailText;
    @FXML private TextField codeField;
    @FXML private Label expirationLabel;
    @FXML private HBox errorBox;
    @FXML private Label errorIconLabel;
    @FXML private Label errorLabel;
    @FXML private HBox infoBox;
    @FXML private Label infoLabel;
    @FXML private Button verifyButton;
    @FXML private Button resendButton;
    @FXML private Hyperlink backToLoginLink;

    public VerifyAccountController(
            RegistrationService registrations,
            BackgroundTasks backgroundTasks,
            ViewManager viewManager) {
        this.registrations = Objects.requireNonNull(registrations);
        this.backgroundTasks = Objects.requireNonNull(backgroundTasks);
        this.viewManager = Objects.requireNonNull(viewManager);
    }

    @FXML
    private void initialize() {
        codeField.setTextFormatter(
                new TextFormatter<String>(
                        change -> change.getControlNewText().matches("\\d{0,8}") ? change : null));

        codeField
                .textProperty()
                .addListener((observable, previous, current) -> AlertBox.hide(errorBox));

        verifyButton
                .sceneProperty()
                .addListener(
                        (observable, previous, current) -> {
                            if (previous != null && current == null) {
                                leave();
                            }
                        });
        refreshAvailability();
    }

    @Override
    public Class<VerifyAccountParams> parametersType() {
        return VerifyAccountParams.class;
    }

    @Override
    public void setParameters(VerifyAccountParams parameters) {
        this.parameters = Objects.requireNonNull(parameters);
        sentToEmailText.setText(parameters.email());

        if (parameters.emailSent()) {
            waitBeforeResend(RESEND_COOLDOWN_SECONDS);
        } else {
            showError(AuthMessages.VERIFICATION_NOT_SENT);
        }

        timer =
                new Timeline(
                        new KeyFrame(
                                javafx.util.Duration.seconds(1), event -> refreshAvailability()));
        timer.setCycleCount(Timeline.INDEFINITE);
        timer.play();

        refreshAvailability();
    }

    @FXML
    private void onVerifyCode() {

        if (!canSubmit()) {
            return;
        }

        String code = codeField.getText();

        if (code.length() != CODE_LENGTH) {
            showError(AuthMessages.CODE_REQUIRED);
            codeField.requestFocus();
            return;
        }

        setBusy(true);

        backgroundTasks.run(
                () -> {
                    registrations.verifyEmail(parameters.attemptId(), code);
                    return null;
                },
                ignored -> onVerified(),
                this::onVerificationFailed);
    }

    private void onVerified() {

        if (!active) {
            return;
        }

        verified = true;
        busy = false;
        stopTimer();
        codeField.clear();
        BusyButton.setBusy(verifyButton, false, null);
        infoLabel.setText(AuthMessages.REGISTRATION_COMPLETE);
        AlertBox.setShown(infoBox, true);
        backToLoginLink.setText("Iniciar sesión");

        refreshAvailability();
    }

    private void onVerificationFailed(Throwable error) {

        if (!active) {
            return;
        }

        setBusy(false);

        if (!(error instanceof RegistrationException)) {
            AuthLog.unexpectedFailure("verify-email", error);
        }

        showError(
                error instanceof RegistrationException failure
                        ? AuthMessages.forVerifyEmail(failure)
                        : AuthMessages.UNEXPECTED);
    }

    @FXML
    private void onResendEmail() {

        if (!canSubmit() || Instant.now().isBefore(resendAvailableAt)) {
            return;
        }

        setBusy(true);

        backgroundTasks.run(
                () -> registrations.resendVerification(parameters.attemptId()),
                this::onResent,
                this::onResendFailed);
    }

    private void onResent(PendingRegistration attempt) {

        if (!active) {
            return;
        }

        setBusy(false);

        codeField.clear();
        waitBeforeResend(RESEND_COOLDOWN_SECONDS);

        if (attempt.emailSent()) {
            infoLabel.setText(AuthMessages.RESEND_DONE);
            AlertBox.setShown(infoBox, true);
        } else {
            showError(AuthMessages.VERIFICATION_NOT_SENT);
        }

        refreshAvailability();
    }

    private void onResendFailed(Throwable error) {

        if (!active) {
            return;
        }

        setBusy(false);

        if (error instanceof RegistrationException failure) {
            if (failure.kind() == RegistrationError.RATE_LIMITED) {
                long seconds =
                        failure.retryAfter()
                                .map(Duration::toSeconds)
                                .filter(wait -> wait > 0)
                                .orElse((long) RESEND_COOLDOWN_SECONDS);
                waitBeforeResend(Math.min(seconds, Integer.MAX_VALUE));
            }
            showError(AuthMessages.forResendVerification(failure));
        } else {
            AuthLog.unexpectedFailure("resend-verification", error);
            showError(AuthMessages.UNEXPECTED);
        }

        refreshAvailability();
    }

    @FXML
    private void onBackToLogin() {
        leave();
        viewManager.show(View.LOGIN);
    }

    @FXML
    private void onRestartRegistration() {
        leave();
        viewManager.show(View.REGISTER);
    }

    private boolean canSubmit() {
        return active
                && parameters != null
                && !busy
                && !verified
                && Instant.now().isBefore(parameters.expiresAt());
    }

    private void setBusy(boolean busy) {
        this.busy = busy;
        AlertBox.hide(errorBox);
        AlertBox.setShown(infoBox, false);
        BusyButton.setBusy(verifyButton, busy, "Procesando…");

        refreshAvailability();
    }

    private void waitBeforeResend(long seconds) {
        resendAvailableAt = Instant.now().plusSeconds(seconds);
    }

    private void refreshAvailability() {
        boolean available = canSubmit();

        codeField.setDisable(!available);
        verifyButton.setDisable(!available);

        long waitSeconds =
                Math.max(0, Duration.between(Instant.now(), resendAvailableAt).getSeconds() + 1);

        resendButton.setDisable(!available || waitSeconds > 0);
        resendButton.setText(waitSeconds > 0 ? cooldownText((int) waitSeconds) : "Reenviar correo");

        updateExpirationLabel();
    }

    private void updateExpirationLabel() {

        if (parameters == null) {
            expirationLabel.setText("Inicia un registro para solicitar un código.");
        } else if (verified) {
            expirationLabel.setText("");
        } else if (!Instant.now().isBefore(parameters.expiresAt())) {
            expirationLabel.setText(AuthMessages.REGISTRATION_EXPIRED);
            stopTimer();
        } else {
            long seconds =
                    Math.max(
                            0,
                            Duration.between(Instant.now(), parameters.expiresAt()).getSeconds());
            expirationLabel.setText(
                    "El código vence en " + seconds / 60 + " min " + seconds % 60 + " s.");
        }
    }

    static String cooldownText(int seconds) {
        return "Reenviar en " + seconds + " s";
    }

    private void showError(String message) {
        AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.ERROR, message);
    }

    private void leave() {
        active = false;
        stopTimer();
        codeField.clear();
    }

    private void stopTimer() {
        if (timer != null) {
            timer.stop();
            timer = null;
        }
    }
}
