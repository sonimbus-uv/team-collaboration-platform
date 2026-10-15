package mx.uv.sonimbus.desktop.ui.auth;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.TextField;
import javafx.scene.layout.HBox;
import javafx.scene.text.Text;
import mx.uv.sonimbus.desktop.ui.navigation.Navigable;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

public class VerifyAccountController implements Navigable {

    private ViewManager viewManager;

    @FXML private Text sentToEmailText;
    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private HBox infoBox;
    @FXML private Label infoLabel;
    @FXML private TextField codeField;
    @FXML private Button verifyButton;
    @FXML private Button resendButton;
    @FXML private Hyperlink backToLoginLink;

    @FXML
    private void initialize() {
        // TODO CU-02: mostrar en sentToEmailText el correo con el que se registró el usuario
        //             (ViewManager no pasa parámetros; obtenerlo de la sesión/estado de registro)
        // TODO CU-02: TextFormatter en codeField para aceptar solo 6 dígitos
        // TODO CU-02: ocultar errorBox e infoBox al editar codeField
    }

    @FXML
    private void onVerify() {
        // TODO CU-02: validar el código; si es incorrecto o caducó, mostrar errorBox;
        //             si es válido, activar la cuenta y ViewManager.show(View.MAIN)
    }

    @FXML
    private void onResendCode() {
        // TODO CU-02: solicitar un nuevo código, mostrar infoBox y deshabilitar
        //             resendButton durante un tiempo de espera
    }

    @FXML
    private void onBackToLogin() {
        viewManager.show(View.LOGIN);
    }

    @Override
    public void setViewManager(ViewManager viewManager) {
        this.viewManager = viewManager;
    }
}
