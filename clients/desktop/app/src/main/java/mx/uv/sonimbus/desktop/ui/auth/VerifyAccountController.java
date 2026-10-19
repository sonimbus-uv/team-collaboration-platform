package mx.uv.sonimbus.desktop.ui.auth;

import java.time.Duration;
import java.util.Objects;
import javafx.animation.KeyFrame;
import javafx.animation.Timeline;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.layout.HBox;
import javafx.scene.text.Text;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.AuthApi;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.navigation.Parameterized;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

/**
 * CU-02 adaptado al enlace: Auth envía un enlace de verificación (no un código) y el usuario lo
 * abre desde su correo. Esta vista informa a dónde se envió y permite reenviarlo.
 */
public class VerifyAccountController implements Parameterized<VerifyAccountParams> {

    /** Igual al cooldown de reenvío de Auth. */
    static final int RESEND_COOLDOWN_SECONDS = 60;
    private static final String RESEND_TEXT = "Reenviar correo";
    private static final String RESEND_ACCESSIBLE_TEXT = "Reenviar correo de verificación";

    private final AuthApi authApi;
    private final BackgroundTasks tasks;
    private final ViewManager viewManager;

    /** Correo recibido como parámetro; {@code null} si la vista se abrió sin él (demostración). */
    private String email;
    private boolean sending;
    private boolean active = true;
    private Timeline cooldown;
    private int cooldownRemaining;

    @FXML private Text sentToEmailText;
    @FXML private HBox errorBox;
    @FXML private Label errorIconLabel;
    @FXML private Label errorLabel;
    @FXML private HBox infoBox;
    @FXML private Label infoLabel;
    @FXML private Button verifyButton;
    @FXML private Button resendButton;
    @FXML private Hyperlink backToLoginLink;

    public VerifyAccountController(AuthApi authApi, BackgroundTasks tasks, ViewManager viewManager) {
        this.authApi = Objects.requireNonNull(authApi);
        this.tasks = Objects.requireNonNull(tasks);
        this.viewManager = Objects.requireNonNull(viewManager);
    }

    @FXML
    private void initialize() {
        infoLabel.setText(AuthMessages.RESEND_DONE);
        // Sin correo no hay a quién reenviar; setParameters lo habilita.
        resendButton.setDisable(true);
    }

    @Override
    public Class<VerifyAccountParams> parametersType() {
        return VerifyAccountParams.class;
    }

    @Override
    public void setParameters(VerifyAccountParams parameters) {
        email = parameters.email();
        sentToEmailText.setText(email);
        resendButton.setDisable(false);
        switch (parameters.delivery()) {
            // Auth acaba de enviar el correo: su cooldown ya corre, así que el botón también espera.
            case SENT -> startCooldown(RESEND_COOLDOWN_SECONDS);
            case FAILED -> AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.ERROR,
                    AuthMessages.VERIFICATION_NOT_SENT);
            // Desde un 409 no se sabe nada del envío: el reenvío queda disponible de inmediato.
            case UNKNOWN -> { }
        }
    }

    @FXML
    private void onResendEmail() {
        if (email == null || sending || cooldownRemaining > 0) {
            return;
        }
        sending = true;
        AlertBox.hide(errorBox);
        AlertBox.setShown(infoBox, false);
        BusyButton.setBusy(resendButton, true, "Enviando…");
        String to = email;
        tasks.run(() -> {
            authApi.resendVerification(to);
            return null;
        }, ignored -> onResent(), this::onResendFailed);
    }

    private void onResent() {
        if (!active) {
            return;
        }
        sending = false;
        BusyButton.setBusy(resendButton, false, null);
        AlertBox.setShown(infoBox, true);
        startCooldown(RESEND_COOLDOWN_SECONDS);
    }

    private void onResendFailed(Throwable error) {
        if (!active) {
            return;
        }
        sending = false;
        BusyButton.setBusy(resendButton, false, null);
        if (!(error instanceof ApiException e)) {
            AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.ERROR, AuthMessages.UNEXPECTED);
            return;
        }
        AlertBox.show(errorBox, errorIconLabel, errorLabel, AlertBox.Kind.ERROR, AuthMessages.forResendVerification(e));
        if (e.kind() == ApiErrorKind.RATE_LIMITED) {
            long wait = e.retryAfter().map(Duration::toSeconds).filter(s -> s > 0).orElse((long) RESEND_COOLDOWN_SECONDS);
            startCooldown((int) Math.min(wait, Integer.MAX_VALUE));
        }
    }

    /** CU-02: tras abrir el enlace, la cuenta queda verificada y el usuario inicia sesión. */
    @FXML
    private void onAlreadyVerified() {
        leave();
        viewManager.show(View.LOGIN);
    }

    @FXML
    private void onBackToLogin() {
        leave();
        viewManager.show(View.LOGIN);
    }

    /** Deshabilita el reenvío y muestra la cuenta regresiva en el botón. */
    private void startCooldown(int seconds) {
        stopCooldown();
        cooldownRemaining = seconds;
        showCooldown();
        cooldown = new Timeline(new KeyFrame(javafx.util.Duration.seconds(1), event -> {
            cooldownRemaining--;
            showCooldown();
        }));
        cooldown.setCycleCount(seconds);
        cooldown.play();
    }

    private void showCooldown() {
        if (cooldownRemaining > 0) {
            resendButton.setDisable(true);
            resendButton.setText(cooldownText(cooldownRemaining));
            resendButton.setAccessibleText(RESEND_ACCESSIBLE_TEXT + ", disponible en " + cooldownRemaining + " segundos");
        } else {
            resendButton.setDisable(false);
            resendButton.setText(RESEND_TEXT);
            resendButton.setAccessibleText(RESEND_ACCESSIBLE_TEXT);
        }
    }

    /** «Reenviar en 42 s». */
    static String cooldownText(int seconds) {
        return "Reenviar en " + seconds + " s";
    }

    private void stopCooldown() {
        if (cooldown != null) {
            cooldown.stop();
            cooldown = null;
        }
        cooldownRemaining = 0;
    }

    private void leave() {
        active = false;
        stopCooldown();
    }
}
