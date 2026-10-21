package mx.uv.sonimbus.desktop.ui.auth;

import java.util.Objects;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.control.ToggleButton;
import javafx.scene.layout.HBox;
import javafx.scene.layout.Region;
import javafx.scene.layout.VBox;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

/**
 * CU-05. Auth todavía no tiene recuperación de contraseña: el paso 1 valida el correo y avisa que
 * no está disponible. El paso 2 queda como esqueleto.
 */
public class RecoverPasswordController {

    private static final String FIELD_INVALID = "field-invalid";

    private final ViewManager viewManager;

    // Indicador de pasos
    @FXML private HBox stepIndicator;
    @FXML private Label step1Badge;
    @FXML private Label step1Label;
    @FXML private Region stepConnector;
    @FXML private Label step2Badge;
    @FXML private Label step2Label;

    // Paso 1
    @FXML private VBox emailStepBox;
    @FXML private TextField emailField;
    @FXML private Button sendCodeButton;
    @FXML private HBox sentInfoBox;
    @FXML private Label sentInfoIconLabel;
    @FXML private Label sentInfoLabel;
    @FXML private Hyperlink backToLoginLink;
    @FXML private Hyperlink haveCodeLink;

    // Paso 2
    @FXML private VBox resetStepBox;
    @FXML private HBox resetErrorBox;
    @FXML private Label resetErrorLabel;
    @FXML private TextField codeField;
    @FXML private Hyperlink resendCodeLink;
    @FXML private PasswordField newPasswordField;
    @FXML private TextField newPasswordVisibleField;
    @FXML private ToggleButton showPasswordToggle;
    @FXML private PasswordField confirmPasswordField;
    @FXML private HBox confirmPasswordErrorBox;
    @FXML private Label confirmPasswordErrorLabel;
    @FXML private Button resetButton;
    @FXML private Hyperlink resetBackToLoginLink;

    public RecoverPasswordController(ViewManager viewManager) {
        this.viewManager = Objects.requireNonNull(viewManager);
    }

    @FXML
    private void initialize() {
        emailField.textProperty().addListener((obs, old, text) -> {
            emailField.getStyleClass().remove(FIELD_INVALID);
            AlertBox.hide(sentInfoBox);
        });
        // TODO CU-05: enlazar newPasswordVisibleField.textProperty() con newPasswordField.textProperty()
        // TODO CU-05: TextFormatter en codeField para aceptar solo 6 dígitos
        // TODO CU-05: ocultar errores y quitar "field-invalid" al editar los campos
    }

    @FXML
    private void onSendCode() {
        if (emailField.getText() == null || emailField.getText().isBlank()) {
            if (!emailField.getStyleClass().contains(FIELD_INVALID)) {
                emailField.getStyleClass().add(FIELD_INVALID);
            }
            AlertBox.show(sentInfoBox, sentInfoIconLabel, sentInfoLabel, AlertBox.Kind.ERROR,
                    "Escribe tu correo electrónico.");
            emailField.requestFocus();
            return;
        }
        // TODO CU-05: cuando Auth tenga el endpoint, validar formato de correo, solicitar código y
        //             mostrar sentInfoBox con el mensaje neutro «Si el correo está registrado,
        //             recibirás un código…» (no revelar si existe); después pasar al paso 2
        showUnavailable();
    }

    @FXML
    private void onShowResetStep() {
        // Sin endpoint de recuperación el paso 2 no sirve todavía.
        showUnavailable();
        // TODO CU-05: cambiar al paso 2:
        //   - emailStepBox oculto / resetStepBox visible (visible + managed)
        //   - sendCodeButton.setDefaultButton(false); resetButton.setDefaultButton(true)
        //   - step1Badge texto "✓" y clase "step-done"; step2Badge/step2Label clase "step-active";
        //     stepConnector clase "step-done"; stepIndicator accessibleText "Paso 2 de 2"
    }

    @FXML
    private void onResendCode() {
        // TODO CU-05: solicitar un nuevo código y deshabilitar resendCodeLink durante un tiempo
    }

    @FXML
    private void onTogglePasswordVisibility() {
        // TODO CU-05: alternar visible/managed entre newPasswordField y newPasswordVisibleField
        //             y cambiar texto a "Ocultar"/"Mostrar"
    }

    @FXML
    private void onResetPassword() {
        // TODO CU-05: validar código, reglas de contraseña y coincidencia (mostrar
        //             confirmPasswordErrorBox / resetErrorBox); si todo es válido,
        //             restablecer y ViewManager.show(View.LOGIN)
    }

    @FXML
    private void onBackToLogin() {
        viewManager.show(View.LOGIN);
    }

    private void showUnavailable() {
        AlertBox.show(sentInfoBox, sentInfoIconLabel, sentInfoLabel, AlertBox.Kind.INFO,
                AuthMessages.RECOVERY_UNAVAILABLE);
    }
}
