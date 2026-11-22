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

    @FXML private VBox formBox;
    @FXML private Label dialogTitleLabel;
    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private TextField titleField;
    @FXML private TextArea descriptionArea;
    @FXML private DatePicker dueDatePicker;
    @FXML private ComboBox<String> statusCombo;
    @FXML private ComboBox<Object> assigneeCombo;
    @FXML private Button deleteButton;
    @FXML private Button cancelButton;
    @FXML private Button saveButton;

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
    }

    @FXML
    private void onSave() {
    }

    @FXML
    private void onDelete() {
    }

    @FXML
    private void onReload() {
    }

    @FXML
    private void onCancel() {
        cancelButton.getScene().getWindow().hide();
    }
}
