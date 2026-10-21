package mx.uv.sonimbus.desktop.ui.auth;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.anyString;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.verifyNoInteractions;
import static org.mockito.Mockito.when;

import java.time.Duration;
import javafx.application.Platform;
import javafx.scene.Node;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.TextInputControl;
import javafx.scene.text.Text;
import javafx.stage.Stage;
import mx.uv.sonimbus.desktop.data.api.Account;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.AuthApi;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.navigation.ControllerFactory;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.testfx.api.FxRobot;
import org.testfx.framework.junit5.ApplicationExtension;
import org.testfx.framework.junit5.Start;
import org.testfx.util.WaitForAsyncUtils;

/** Pantallas de autenticación con JavaFX real (Monocle sin pantalla) y Auth simulado. */
@ExtendWith(ApplicationExtension.class)
class AuthScreensTest {

    private static final String EMAIL = "ana@uv.mx";
    private static final String PASSWORD = "ValidPass123!";

    private final AuthApi authApi = mock(AuthApi.class);
    private Stage stage;
    private ViewManager viewManager;

    @Start
    void start(Stage stage) {
        this.stage = stage;
        ControllerFactory controllers = new ControllerFactory();
        viewManager = new ViewManager(stage, controllers);
        controllers
                .register(AuthApi.class, authApi)
                // El trabajo corre en el mismo hilo; la respuesta llega con runLater, como en la app.
                .register(BackgroundTasks.class, new BackgroundTasks(Runnable::run, Platform::runLater))
                .register(ViewManager.class, viewManager);
    }

    // --- Registro (CU-01) ---

    @Test
    void registerScreenShowsTheRealRuleAndHidesName(FxRobot robot) {
        open(robot, View.REGISTER, null);

        assertEquals(AuthMessages.PASSWORD_RULE, label(robot, "#passwordHintLabel").getText());
        assertFalse(node(robot, "#nameField").isVisible());
    }

    @Test
    void successfulRegistrationNavigatesToVerificationWithTheEmail(FxRobot robot) throws ApiException {
        when(authApi.register(EMAIL, PASSWORD)).thenReturn(new Account("7b0c", EMAIL, false, "active", true));
        open(robot, View.REGISTER, null);

        fillRegistration(robot, "  Ana@UV.mx ", PASSWORD, PASSWORD);
        robot.clickOn("#registerButton");
        WaitForAsyncUtils.waitForFxEvents();

        verify(authApi).register(EMAIL, PASSWORD);
        assertEquals(EMAIL, robot.lookup("#sentToEmailText").queryAs(Text.class).getText());
        Button resend = robot.lookup("#resendButton").queryButton();
        assertTrue(resend.isDisabled(), "el reenvío espera el cooldown de Auth");
        assertTrue(resend.getText().startsWith("Reenviar en "), resend.getText());
    }

    @Test
    void takenEmailShowsMessageAndLeadsToResend(FxRobot robot) throws ApiException {
        when(authApi.register(EMAIL, PASSWORD))
                .thenThrow(new ApiException(ApiErrorKind.EMAIL_TAKEN, "register", 409, null));
        open(robot, View.REGISTER, null);

        fillRegistration(robot, EMAIL, PASSWORD, PASSWORD);
        robot.clickOn("#registerButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals("Ya existe una cuenta con este correo.", label(robot, "#errorLabel").getText());
        assertTrue(field(robot, "#emailField").getStyleClass().contains("field-invalid"));
        assertTrue(node(robot, "#resendVerificationLink").isVisible());

        robot.clickOn("#resendVerificationLink");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals(EMAIL, robot.lookup("#sentToEmailText").queryAs(Text.class).getText());
        assertFalse(node(robot, "#errorBox").isVisible(), "no se afirma que el envío falló");
        assertFalse(robot.lookup("#resendButton").queryButton().isDisabled());
    }

    @Test
    void invalidFormIsNotSent(FxRobot robot) {
        open(robot, View.REGISTER, null);

        fillRegistration(robot, "ana@", "corta", "otra");
        robot.clickOn("#registerButton");
        WaitForAsyncUtils.waitForFxEvents();

        verifyNoInteractions(authApi);
        String message = label(robot, "#errorLabel").getText();
        assertTrue(message.contains("El correo no tiene un formato válido."), message);
        assertTrue(message.contains("La contraseña necesita al menos 8 caracteres"), message);
        assertTrue(node(robot, "#confirmPasswordErrorBox").isVisible());
        assertTrue(field(robot, "#passwordField").getStyleClass().contains("field-invalid"));
    }

    @Test
    void connectionErrorShowsSpanishMessage(FxRobot robot) throws ApiException {
        when(authApi.register(EMAIL, PASSWORD))
                .thenThrow(new ApiException(ApiErrorKind.TIMEOUT, "register", new java.net.http.HttpTimeoutException("x")));
        open(robot, View.REGISTER, null);

        fillRegistration(robot, EMAIL, PASSWORD, PASSWORD);
        robot.clickOn("#registerButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals("No se pudo conectar con el servidor. Revisa tu conexión e intenta de nuevo.",
                label(robot, "#errorLabel").getText());
        Button register = robot.lookup("#registerButton").queryButton();
        assertFalse(register.isDisabled(), "se puede reintentar");
        assertEquals("Crear cuenta", register.getText());
    }

    @Test
    void googleRegistrationIsNotAvailableYet(FxRobot robot) {
        open(robot, View.REGISTER, null);

        robot.clickOn("#googleButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals(AuthMessages.GOOGLE_UNAVAILABLE, label(robot, "#errorLabel").getText());
        assertTrue(node(robot, "#errorBox").getStyleClass().contains("alert-info"));
    }

    // --- Verificación (CU-02) ---

    @Test
    void failedDeliveryWarnsAndAllowsResend(FxRobot robot) throws ApiException {
        open(robot, View.VERIFY_ACCOUNT, VerifyAccountParams.afterRegistration(EMAIL, false));

        assertEquals(AuthMessages.VERIFICATION_NOT_SENT, label(robot, "#errorLabel").getText());
        assertFalse(robot.lookup("#resendButton").queryButton().isDisabled());

        robot.clickOn("#resendButton");
        WaitForAsyncUtils.waitForFxEvents();

        verify(authApi).resendVerification(EMAIL);
        assertTrue(node(robot, "#infoBox").isVisible());
        assertEquals(AuthMessages.RESEND_DONE, label(robot, "#infoLabel").getText());
        assertFalse(node(robot, "#errorBox").isVisible());
        Button resend = robot.lookup("#resendButton").queryButton();
        assertTrue(resend.isDisabled());
        assertTrue(resend.getText().startsWith("Reenviar en "), resend.getText());
    }

    @Test
    void rateLimitedResendWaitsWhatTheServerSays(FxRobot robot) throws ApiException {
        doThrow(new ApiException(ApiErrorKind.RATE_LIMITED, "resend-verification", 429, Duration.ofSeconds(30)))
                .when(authApi).resendVerification(anyString());
        open(robot, View.VERIFY_ACCOUNT, VerifyAccountParams.forExistingAccount(EMAIL));

        robot.clickOn("#resendButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals("Demasiadas solicitudes. Intenta de nuevo en 30 segundos.", label(robot, "#errorLabel").getText());
        Button resend = robot.lookup("#resendButton").queryButton();
        assertTrue(resend.isDisabled());
        assertTrue(resend.getText().matches("Reenviar en (30|29) s"), resend.getText());
    }

    @Test
    void verificationWithoutParametersCannotResend(FxRobot robot) {
        open(robot, View.VERIFY_ACCOUNT, null);

        assertEquals("tu correo", robot.lookup("#sentToEmailText").queryAs(Text.class).getText());
        assertTrue(robot.lookup("#resendButton").queryButton().isDisabled());
    }

    @Test
    void alreadyVerifiedGoesToLogin(FxRobot robot) {
        open(robot, View.VERIFY_ACCOUNT, VerifyAccountParams.forExistingAccount(EMAIL));

        robot.clickOn("#verifyButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertTrue(robot.lookup("#loginButton").tryQuery().isPresent());
    }

    // --- Inicio de sesión (CU-03) y recuperación (CU-05) ---

    @Test
    void loginRequiresBothFields(FxRobot robot) {
        open(robot, View.LOGIN, null);

        robot.clickOn("#loginButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals(AuthMessages.LOGIN_FIELDS_EMPTY, label(robot, "#errorLabel").getText());
        assertTrue(node(robot, "#errorBox").getStyleClass().contains("alert-error"));
        assertTrue(field(robot, "#emailField").getStyleClass().contains("field-invalid"));
    }

    @Test
    void loginIsNotAvailableYet(FxRobot robot) {
        open(robot, View.LOGIN, null);

        robot.interact(() -> {
            field(robot, "#emailField").setText(EMAIL);
            field(robot, "#passwordField").setText(PASSWORD);
        });
        robot.clickOn("#loginButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals(AuthMessages.LOGIN_UNAVAILABLE, label(robot, "#errorLabel").getText());
        Node box = node(robot, "#errorBox");
        assertTrue(box.getStyleClass().contains("alert-info"));
        assertFalse(box.getStyleClass().contains("alert-error"));
        assertEquals("i", label(robot, "#errorIconLabel").getText(), "se distingue por icono, no solo color");
        verifyNoInteractions(authApi);
    }

    @Test
    void passwordCanBeShown(FxRobot robot) {
        open(robot, View.LOGIN, null);
        robot.interact(() -> field(robot, "#passwordField").setText(PASSWORD));

        robot.clickOn("#showPasswordToggle");
        WaitForAsyncUtils.waitForFxEvents();

        assertTrue(node(robot, "#passwordVisibleField").isVisible());
        assertEquals(PASSWORD, field(robot, "#passwordVisibleField").getText());
        assertEquals("Ocultar", robot.lookup("#showPasswordToggle").queryLabeled().getText());
    }

    @Test
    void recoveryIsNotAvailableYet(FxRobot robot) {
        open(robot, View.RECOVER_PASSWORD, null);
        robot.interact(() -> field(robot, "#emailField").setText(EMAIL));

        robot.clickOn("#sendCodeButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals(AuthMessages.RECOVERY_UNAVAILABLE, label(robot, "#sentInfoLabel").getText());
    }

    // --- Utilidades ---

    private void open(FxRobot robot, View view, Object parameters) {
        robot.interact(() -> {
            viewManager.show(view, parameters);
            stage.show();
            stage.toFront();
        });
        WaitForAsyncUtils.waitForFxEvents();
    }

    private void fillRegistration(FxRobot robot, String email, String password, String confirmation) {
        robot.interact(() -> {
            field(robot, "#emailField").setText(email);
            field(robot, "#passwordField").setText(password);
            field(robot, "#confirmPasswordField").setText(confirmation);
        });
    }

    private static TextInputControl field(FxRobot robot, String query) {
        return robot.lookup(query).queryAs(TextInputControl.class);
    }

    private static Label label(FxRobot robot, String query) {
        return robot.lookup(query).queryAs(Label.class);
    }

    private static Node node(FxRobot robot, String query) {
        return robot.lookup(query).query();
    }
}
