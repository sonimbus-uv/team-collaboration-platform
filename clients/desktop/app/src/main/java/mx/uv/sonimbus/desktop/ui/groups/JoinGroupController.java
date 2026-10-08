package mx.uv.sonimbus.desktop.ui.groups;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.TextField;
import javafx.scene.layout.HBox;

public class JoinGroupController {

    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private TextField inviteCodeField;
    @FXML private HBox groupPreviewBox;
    @FXML private Label groupInitialsLabel;
    @FXML private Label groupNameLabel;
    @FXML private Label groupDetailsLabel;
    @FXML private Button cancelButton;
    @FXML private Button joinButton;

    @FXML
    private void initialize() {
        // TODO CU-11: si se pega un enlace de invitación completo, extraer el código
        // TODO CU-11: al completar el código, consultar la invitación y mostrar groupPreviewBox
        //             (iniciales, nombre, "N miembros · Invitación válida")
        // TODO CU-11: ocultar errorBox y quitar "field-invalid" de inviteCodeField al editar
    }

    @FXML
    private void onJoinGroup() {
        // TODO CU-11: validar el código; si la invitación es inválida, caducada, revocada o
        //             agotada mostrar errorBox; si es válida, unirse (sin duplicar la membresía),
        //             cerrar el diálogo y seleccionar el grupo en el shell
    }

    @FXML
    private void onCancel() {
        // TODO CU-11: cerrar el diálogo (Stage) sin unirse
    }
}
