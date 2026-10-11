package mx.uv.sonimbus.desktop.ui.auth;

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

public class RecoverPasswordController {

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

    @FXML
    private void initialize() {
        // TODO CU-05: enlazar newPasswordVisibleField.textProperty() con newPasswordField.textProperty()
        // TODO CU-05: TextFormatter en codeField para aceptar solo 6 dígitos
        // TODO CU-05: ocultar errores y quitar "field-invalid" al editar los campos
    }

    @FXML
    private void onSendCode() {
        // TODO CU-05: validar formato de correo, solicitar código y mostrar sentInfoBox
        //             (mensaje neutro: no revelar si el correo existe); después pasar al paso 2
    }

    @FXML
    private void onShowResetStep() {
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
        // TODO CU-03: ViewManager.show(View.LOGIN)
    }
}
