package mx.uv.sonimbus.desktop.ui.projects;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.ComboBox;
import javafx.scene.control.DatePicker;
import javafx.scene.control.Label;
import javafx.scene.control.TextArea;
import javafx.scene.control.TextField;
import javafx.scene.layout.HBox;
import javafx.scene.layout.VBox;

public class TaskDetailController {

    // Formulario
    @FXML private VBox formBox;
    @FXML private Label dialogTitleLabel;
    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private TextField titleField;
    @FXML private TextArea descriptionArea;
    @FXML private DatePicker dueDatePicker;
    @FXML private ComboBox<String> statusCombo;
    @FXML private ComboBox<Object> assigneeCombo;        // TODO: modelo de miembro
    @FXML private Button deleteButton;
    @FXML private Button cancelButton;
    @FXML private Button saveButton;

    // Conflicto de edición
    @FXML private VBox conflictBox;
    @FXML private Label conflictLabel;
    @FXML private Label currentStatusLabel;
    @FXML private Label currentAssigneeLabel;
    @FXML private Label currentDueDateLabel;
    @FXML private Label yourStatusLabel;
    @FXML private Label yourAssigneeLabel;
    @FXML private Label yourDueDateLabel;
    @FXML private Button conflictCancelButton;
    @FXML private Button reloadButton;

    @FXML
    private void initialize() {
        // TODO CU-26: tarea nueva -> dialogTitleLabel "Nueva tarea", ocultar deleteButton y
        //             seleccionar el estado inicial; tarea existente -> cargar sus datos
        // TODO CU-27: llenar assigneeCombo con "Sin asignar" + los miembros del grupo;
        //             deshabilitarlo si el usuario no es administrador
        // TODO CU-28: cell factory de statusCombo con icono + texto ("status-chip",
        //             "status-active", "status-success"); habilitarlo solo para el responsable
        //             o el administrador
        // TODO CU-26: ocultar errorBox y quitar "field-invalid" de titleField al editar
    }

    @FXML
    private void onSave() {
        // TODO CU-26, CU-27, CU-28: validar titleField (errorBox) y guardar enviando la versión
        //             de la tarea; si el servidor detecta un cambio concurrente, mostrar el
        //             conflicto: llenar conflictLabel ("<miembro> guardó cambios en «<tarea>»
        //             a las hh:mm, mientras la editabas. Tus cambios no se guardaron.") y las
        //             etiquetas current*/your*, ocultar formBox, mostrar conflictBox
        //             (visible + managed), saveButton.setDefaultButton(false),
        //             reloadButton.setDefaultButton(true), cancelButton.setCancelButton(false),
        //             conflictCancelButton.setCancelButton(true) y ajustar el tamaño del Stage
        //             (sizeToScene)
    }

    @FXML
    private void onDelete() {
        // TODO CU-26: pedir confirmación, eliminar la tarea y cerrar el diálogo
    }

    @FXML
    private void onReload() {
        // TODO CU-26: recargar la versión vigente de la tarea en el formulario, ocultar
        //             conflictBox, mostrar formBox y restaurar defaultButton/cancelButton
    }

    @FXML
    private void onCancel() {
        // TODO CU-26: cerrar el diálogo (Stage) sin guardar
    }
}
