package mx.uv.sonimbus.desktop.ui.auth;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;
import static org.mockito.ArgumentMatchers.any;
import static org.mockito.Mockito.doThrow;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.verify;
import static org.mockito.Mockito.verifyNoInteractions;
import static org.mockito.Mockito.when;

import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;

import javafx.application.Platform;
import javafx.scene.Node;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.TextInputControl;
import javafx.scene.text.Text;
import javafx.stage.Stage;

import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;
import mx.uv.sonimbus.desktop.data.api.auth.google.GoogleAttemptResponse;
import mx.uv.sonimbus.desktop.data.api.auth.google.HttpGoogleGateway;
import mx.uv.sonimbus.desktop.data.api.auth.password.HttpPasswordGateway;
import mx.uv.sonimbus.desktop.data.api.auth.registration.Account;
import mx.uv.sonimbus.desktop.data.api.auth.registration.HttpRegistrationGateway;
import mx.uv.sonimbus.desktop.data.api.auth.registration.RegistrationAttempt;
import mx.uv.sonimbus.desktop.data.api.auth.session.HttpAuthenticationGateway;
import mx.uv.sonimbus.desktop.data.api.auth.session.TokenResponse;
import mx.uv.sonimbus.desktop.domain.google.GoogleBrowser;
import mx.uv.sonimbus.desktop.domain.google.GoogleError;
import mx.uv.sonimbus.desktop.domain.google.GoogleException;
import mx.uv.sonimbus.desktop.domain.google.GoogleService;
import mx.uv.sonimbus.desktop.domain.password.PasswordService;
import mx.uv.sonimbus.desktop.domain.registration.PendingRegistration;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationService;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.ui.auth.common.AuthMessages;
import mx.uv.sonimbus.desktop.ui.auth.registration.VerifyAccountParams;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.navigation.ControllerFactory;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;
import mx.uv.sonimbus.desktop.ui.theme.ThemeManager;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.testfx.api.FxRobot;
import org.testfx.framework.junit5.ApplicationExtension;
import org.testfx.framework.junit5.Start;
import org.testfx.util.WaitForAsyncUtils;

import java.time.Duration;
import java.time.Instant;
import java.util.UUID;

/** Pantallas de autenticación con JavaFX real (Monocle sin pantalla) y Auth simulado. */
@ExtendWith(ApplicationExtension.class)
class AuthScreensTest {

    private static final String EMAIL = "ana@uv.mx";
    private static final String PASSWORD = "ValidPass123!";

    private final AuthApi authApi = mock(AuthApi.class);
    private Stage stage;
    private ViewManager viewManager;
    private final SessionService sessions =
            new SessionService(new HttpAuthenticationGateway(authApi));

    private final GoogleBrowser googleBrowser = mock(GoogleBrowser.class);
    private final GoogleService google =
            new GoogleService(new HttpGoogleGateway(authApi), googleBrowser, sessions);

    @Start
    void start(Stage stage) throws Exception {
        when(authApi.googleConnections(any()))
                .thenReturn(
                        new mx.uv.sonimbus.desktop.data.api.auth.google.GoogleConnectionsResponse(
                                java.util.List.of()));
        this.stage = stage;
        ControllerFactory controllers = new ControllerFactory();
        viewManager =
                new ViewManager(stage, controllers, new ThemeManager(), sessions::isAuthenticated);
        controllers
                .register(GoogleService.class, google)
                .register(
                        RegistrationService.class,
                        new RegistrationService(new HttpRegistrationGateway(authApi)))
                .register(SessionService.class, sessions)
                .register(
                        PasswordService.class,
                        new PasswordService(new HttpPasswordGateway(authApi), sessions))
                // El trabajo corre en el mismo hilo; la respuesta llega con runLater, como en la
                // app.
                .register(
                        BackgroundTasks.class,
                        new BackgroundTasks(Runnable::run, Platform::runLater))
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
    void successfulRegistrationNavigatesToVerificationWithTheEmail(FxRobot robot)
            throws ApiException {
        when(authApi.register(EMAIL, PASSWORD)).thenReturn(pendingAttempt(true));
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
    void takenEmailDoesNotOfferResendWithoutAnAttempt(FxRobot robot) throws ApiException {
        when(authApi.register(EMAIL, PASSWORD))
                .thenThrow(new ApiException(ApiErrorKind.EMAIL_TAKEN, "register", 409, null));
        open(robot, View.REGISTER, null);
        fillRegistration(robot, EMAIL, PASSWORD, PASSWORD);
        robot.clickOn("#registerButton");
        WaitForAsyncUtils.waitForFxEvents();
        assertEquals(AuthMessages.EMAIL_TAKEN, label(robot, "#errorLabel").getText());
        assertTrue(robot.lookup("#resendVerificationLink").tryQuery().isEmpty());
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
                .thenThrow(
                        new ApiException(
                                ApiErrorKind.TIMEOUT,
                                "register",
                                new java.net.http.HttpTimeoutException("x")));
        open(robot, View.REGISTER, null);

        fillRegistration(robot, EMAIL, PASSWORD, PASSWORD);
        robot.clickOn("#registerButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals(
                "No se pudo conectar con el servidor. Revisa tu conexión e intenta de nuevo.",
                label(robot, "#errorLabel").getText());
        Button register = robot.lookup("#registerButton").queryButton();
        assertFalse(register.isDisabled(), "se puede reintentar");
        assertEquals("Crear cuenta", register.getText());
    }

    @Test
    void disabledGoogleProviderShowsRetryableError(FxRobot robot) throws Exception {
        when(authApi.createGoogleAttempt(any()))
                .thenThrow(new ApiException(ApiErrorKind.UNAVAILABLE, "google-attempt", 503, null));
        open(robot, View.REGISTER, null);

        robot.clickOn("#googleButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertTrue(label(robot, "#errorLabel").getText().contains("no está disponible"));
        assertTrue(node(robot, "#errorBox").getStyleClass().contains("alert-error"));
    }

    @Test
    void googleLoginAndRegistrationOpenMainWithoutRegistrationCode(FxRobot robot) throws Exception {
        String secret = "A".repeat(43);
        when(authApi.createGoogleAttempt(any()))
                .thenReturn(
                        new GoogleAttemptResponse(
                                ATTEMPT_ID,
                                secret,
                                "https://auth.invalid/api/v1/auth/google/authorize?ticket="
                                        + secret,
                                Instant.now().plusSeconds(600).toString()));
        when(googleBrowser.authorize(any(), any())).thenReturn(secret);
        when(authApi.exchangeGoogle(any())).thenReturn(tokenResponse());
        for (View view : new View[] {View.LOGIN, View.REGISTER}) {
            sessions.invalidate();
            open(robot, view, null);
            robot.interact(() -> robot.lookup("#googleButton").queryButton().fire());
            WaitForAsyncUtils.waitForFxEvents();
            assertTrue(sessions.isAuthenticated());
            assertTrue(sessions.email().isEmpty());
            assertTrue(robot.lookup("#groupsList").tryQuery().isPresent());
        }
        verify(authApi, org.mockito.Mockito.never()).register(any(), any());
        verify(authApi, org.mockito.Mockito.never()).verifyEmail(any(), any());
    }

    @Test
    void googleCancellationRestoresButtonsWithoutInstallingSession(FxRobot robot) throws Exception {
        String secret = "A".repeat(43);
        when(authApi.createGoogleAttempt(any()))
                .thenReturn(
                        new GoogleAttemptResponse(
                                ATTEMPT_ID,
                                secret,
                                "https://auth.invalid/api/v1/auth/google/authorize?ticket="
                                        + secret,
                                Instant.now().plusSeconds(600).toString()));
        when(googleBrowser.authorize(any(), any()))
                .thenThrow(new GoogleException(GoogleError.CANCELLED));
        open(robot, View.LOGIN, null);
        robot.interact(() -> robot.lookup("#googleButton").queryButton().fire());
        WaitForAsyncUtils.waitForFxEvents();
        assertFalse(sessions.isAuthenticated());
        assertFalse(robot.lookup("#googleButton").queryButton().isDisabled());
        assertFalse(node(robot, "#cancelGoogleButton").isVisible());
        assertTrue(label(robot, "#errorLabel").getText().contains("canceló"));
        verify(authApi, org.mockito.Mockito.never()).exchangeGoogle(any());
    }

    // --- Verificación (CU-02) ---

    @Test
    void failedDeliveryWarnsAndAllowsResend(FxRobot robot) throws ApiException {
        when(authApi.resendVerification(ATTEMPT_ID)).thenReturn(pendingAttempt(true));
        open(robot, View.VERIFY_ACCOUNT, verificationParams(false));

        assertEquals(AuthMessages.VERIFICATION_NOT_SENT, label(robot, "#errorLabel").getText());
        assertFalse(robot.lookup("#resendButton").queryButton().isDisabled());

        robot.clickOn("#resendButton");
        WaitForAsyncUtils.waitForFxEvents();

        verify(authApi).resendVerification(ATTEMPT_ID);
        assertTrue(node(robot, "#infoBox").isVisible());
        assertEquals(AuthMessages.RESEND_DONE, label(robot, "#infoLabel").getText());
        assertFalse(node(robot, "#errorBox").isVisible());
        Button resend = robot.lookup("#resendButton").queryButton();
        assertTrue(resend.isDisabled());
        assertTrue(resend.getText().startsWith("Reenviar en "), resend.getText());
    }

    @Test
    void rateLimitedResendWaitsWhatTheServerSays(FxRobot robot) throws ApiException {
        doThrow(
                        new ApiException(
                                ApiErrorKind.RATE_LIMITED,
                                "resend-verification",
                                429,
                                Duration.ofSeconds(30)))
                .when(authApi)
                .resendVerification(any());
        open(robot, View.VERIFY_ACCOUNT, verificationParams(false));

        robot.clickOn("#resendButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals(
                "Demasiadas solicitudes. Intenta de nuevo en 30 segundos.",
                label(robot, "#errorLabel").getText());
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
    void correctCodeCompletesRegistrationAndOffersLogin(FxRobot robot) throws ApiException {
        when(authApi.register(EMAIL, PASSWORD)).thenReturn(pendingAttempt(true));
        when(authApi.verifyEmail(ATTEMPT_ID, "00123456"))
                .thenReturn(new Account("7b0c", EMAIL, true, "active"));
        open(robot, View.REGISTER, null);
        fillRegistration(robot, EMAIL, PASSWORD, PASSWORD);
        robot.clickOn("#registerButton");
        WaitForAsyncUtils.waitForFxEvents();
        robot.interact(() -> field(robot, "#codeField").setText("00123456"));
        robot.clickOn("#verifyButton");
        WaitForAsyncUtils.waitForFxEvents();
        verify(authApi).verifyEmail(ATTEMPT_ID, "00123456");
        assertEquals(AuthMessages.REGISTRATION_COMPLETE, label(robot, "#infoLabel").getText());
        robot.clickOn("#backToLoginLink");
        assertTrue(robot.lookup("#loginButton").tryQuery().isPresent());
    }

    @Test
    void expiredAttemptCannotVerifyOrResend(FxRobot robot) {
        open(
                robot,
                View.VERIFY_ACCOUNT,
                new VerifyAccountParams(EMAIL, ATTEMPT_ID, Instant.now().minusSeconds(1), true));
        assertTrue(robot.lookup("#verifyButton").queryButton().isDisabled());
        assertTrue(robot.lookup("#resendButton").queryButton().isDisabled());
        assertEquals(AuthMessages.REGISTRATION_EXPIRED, label(robot, "#expirationLabel").getText());
        verifyNoInteractions(authApi);
    }

    @Test
    void incompleteCodeIsNotSubmitted(FxRobot robot) {
        open(robot, View.VERIFY_ACCOUNT, verificationParams(true));
        robot.interact(() -> field(robot, "#codeField").setText("123"));
        robot.clickOn("#verifyButton");
        assertEquals(AuthMessages.CODE_REQUIRED, label(robot, "#errorLabel").getText());
        verifyNoInteractions(authApi);
    }

    @Test
    void rejectedCodeKeepsTheAttemptAvailableForCorrection(FxRobot robot) throws ApiException {
        when(authApi.verifyEmail(ATTEMPT_ID, "12345678"))
                .thenThrow(new ApiException(ApiErrorKind.VALIDATION, "verify-email", 400, null));
        open(robot, View.VERIFY_ACCOUNT, verificationParams(true));
        robot.interact(() -> field(robot, "#codeField").setText("12345678"));
        robot.clickOn("#verifyButton");
        WaitForAsyncUtils.waitForFxEvents();
        assertEquals(AuthMessages.INVALID_CODE, label(robot, "#errorLabel").getText());
        assertFalse(robot.lookup("#verifyButton").queryButton().isDisabled());
        assertFalse(field(robot, "#codeField").isDisabled());
    }

    @Test
    void codeFieldRejectsNonAsciiDigitsAndMoreThanEightCharacters(FxRobot robot) {
        open(robot, View.VERIFY_ACCOUNT, verificationParams(true));
        robot.interact(
                () -> {
                    field(robot, "#codeField").setText("00123456");
                    field(robot, "#codeField").appendText("9");
                    assertEquals("00123456", field(robot, "#codeField").getText());
                    field(robot, "#codeField").setText("not-code");
                    assertEquals("00123456", field(robot, "#codeField").getText());
                });
        verifyNoInteractions(authApi);
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
    void loginRejectsInvalidCredentialsWithoutExposingServerDetails(FxRobot robot)
            throws ApiException {
        when(authApi.login(EMAIL, PASSWORD))
                .thenThrow(new ApiException(ApiErrorKind.INVALID_CREDENTIALS, "login", 401, null));
        open(robot, View.LOGIN, null);

        robot.interact(
                () -> {
                    field(robot, "#emailField").setText(EMAIL);
                    field(robot, "#passwordField").setText(PASSWORD);
                });
        robot.clickOn("#loginButton");
        WaitForAsyncUtils.waitForFxEvents();

        assertEquals(AuthMessages.INVALID_LOGIN, label(robot, "#errorLabel").getText());
        Node box = node(robot, "#errorBox");
        assertTrue(box.getStyleClass().contains("alert-error"));
        assertEquals(
                "!",
                label(robot, "#errorIconLabel").getText(),
                "se distingue por icono, no solo color");
        verify(authApi).login(EMAIL, PASSWORD);
        assertFalse(sessions.isAuthenticated());
    }

    @Test
    void protectedViewsAndDialogsRequireALogin(FxRobot robot) {
        open(robot, View.MAIN, null);
        assertTrue(robot.lookup("#loginButton").tryQuery().isPresent());
        robot.interact(() -> viewManager.showContent(View.PROFILE));
        assertTrue(robot.lookup("#loginButton").tryQuery().isPresent());
        robot.interact(() -> viewManager.showDialog(View.CREATE_GROUP, "Crear grupo"));
        assertTrue(robot.lookup("#loginButton").tryQuery().isPresent());
        verifyNoInteractions(authApi);
    }

    private static TokenResponse tokenResponse() throws Exception {
        return new ObjectMapper()
                .setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
                .readValue(
                        """
{"access_token":"private-access-token","refresh_token":"private-refresh-token",
"token_type":"Bearer","access_expires_in":900,
"session_expires_at":"2030-01-01T00:00:00Z",
"sid":"2f862636-cf2f-4c18-820e-7c29dc449ba0",
"account_id":"3f862636-cf2f-4c18-820e-7c29dc449ba0"}
""",
                        TokenResponse.class);
    }

    @Test
    void confirmationPasswordCanBeShownAndEditedIndependently(FxRobot robot) {
        open(robot, View.REGISTER, null);
        fillRegistration(robot, EMAIL, PASSWORD, "Different123!");

        robot.clickOn("#showConfirmPasswordToggle");
        WaitForAsyncUtils.waitForFxEvents();

        assertTrue(node(robot, "#confirmPasswordVisibleField").isVisible());
        assertFalse(node(robot, "#confirmPasswordField").isVisible());
        assertFalse(node(robot, "#passwordVisibleField").isVisible());
        assertEquals("Different123!", field(robot, "#confirmPasswordVisibleField").getText());
        assertEquals(
                "Ocultar", robot.lookup("#showConfirmPasswordToggle").queryLabeled().getText());

        robot.interact(() -> robot.lookup("#registerButton").queryButton().fire());
        assertTrue(node(robot, "#confirmPasswordErrorBox").isVisible());
        assertTrue(
                field(robot, "#confirmPasswordVisibleField")
                        .getStyleClass()
                        .contains("field-invalid"));

        robot.interact(() -> field(robot, "#confirmPasswordVisibleField").setText(PASSWORD));
        assertEquals(PASSWORD, field(robot, "#confirmPasswordField").getText());
        assertFalse(node(robot, "#confirmPasswordErrorBox").isVisible());
        assertFalse(
                field(robot, "#confirmPasswordVisibleField")
                        .getStyleClass()
                        .contains("field-invalid"));

        robot.clickOn("#showConfirmPasswordToggle");
        assertTrue(node(robot, "#confirmPasswordField").isVisible());
        assertFalse(node(robot, "#confirmPasswordVisibleField").isVisible());
        assertEquals(PASSWORD, field(robot, "#confirmPasswordField").getText());
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

    private void fillPasswordChange(FxRobot robot) {
        robot.interact(
                () -> {
                    field(robot, "#currentPasswordField").setText(PASSWORD);
                    robot.lookup("#showCurrentPasswordToggle")
                            .queryAs(javafx.scene.control.ToggleButton.class)
                            .fire();
                    assertEquals(PASSWORD, field(robot, "#currentPasswordVisibleField").getText());
                    assertFalse(node(robot, "#newPasswordVisibleField").isVisible());
                    assertFalse(node(robot, "#confirmPasswordVisibleField").isVisible());
                    robot.lookup("#showNewPasswordToggle")
                            .queryAs(javafx.scene.control.ToggleButton.class)
                            .fire();
                    robot.lookup("#showConfirmPasswordToggle")
                            .queryAs(javafx.scene.control.ToggleButton.class)
                            .fire();
                    field(robot, "#newPasswordVisibleField").setText("DifferentPass456!");
                    field(robot, "#confirmPasswordVisibleField").setText("DifferentPass456!");
                });
    }

    private GoogleAttemptResponse linkAttempt() {
        String secret = "A".repeat(43);
        return new GoogleAttemptResponse(
                ATTEMPT_ID,
                secret,
                "https://auth.invalid/api/v1/auth/google/authorize?ticket=" + secret,
                Instant.now().plusSeconds(600).toString());
    }

    private void signInForLink() throws Exception {
        when(authApi.login(EMAIL, PASSWORD)).thenReturn(tokenResponse());
        sessions.login(new mx.uv.sonimbus.desktop.domain.session.LoginCredentials(EMAIL, PASSWORD));
        when(authApi.createGoogleLinkAttempt(any(), any())).thenReturn(linkAttempt());
        when(googleBrowser.authorize(any(), any())).thenReturn("A".repeat(43));
    }

    private static mx.uv.sonimbus.desktop.data.api.auth.google.GoogleConnectionsResponse
            linkedGoogleResponse() {
        return new mx.uv.sonimbus.desktop.data.api.auth.google.GoogleConnectionsResponse(
                java.util.List.of(
                        new mx.uv.sonimbus.desktop.data.api.auth.google.GoogleConnectionsResponse
                                .Connection(
                                "google", "reference@gmail.com", Instant.now().toString())));
    }

    // --- Utilidades ---

    private static final UUID ATTEMPT_ID = UUID.fromString("2f862636-cf2f-4c18-820e-7c29dc449ba0");

    private static VerifyAccountParams verificationParams(boolean sent) {
        return VerifyAccountParams.afterRegistration(
                EMAIL, new PendingRegistration(ATTEMPT_ID, Instant.now().plusSeconds(600), sent));
    }

    private static RegistrationAttempt pendingAttempt(boolean sent) {
        return new RegistrationAttempt(ATTEMPT_ID, Instant.now().plusSeconds(600).toString(), sent);
    }

    private void open(FxRobot robot, View view, Object parameters) {
        robot.interact(
                () -> {
                    viewManager.show(view, parameters);
                    stage.show();
                    stage.toFront();
                });
        WaitForAsyncUtils.waitForFxEvents();
    }

    private void fillRegistration(
            FxRobot robot, String email, String password, String confirmation) {
        robot.interact(
                () -> {
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
