package mx.uv.sonimbus.desktop.ui.groups;

import java.util.Objects;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.TextField;
import javafx.scene.layout.HBox;
import mx.uv.sonimbus.desktop.data.core.CoreApi;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.common.CoreMessages;
import mx.uv.sonimbus.desktop.ui.navigation.Parameterized;

public class JoinGroupController implements Parameterized<GroupDialogParams> {

    private static final String FIELD_INVALID = "field-invalid";
    static final int CODE_MAX = 64;

    private final CoreApi core;
    private final BackgroundTasks tasks;
    private GroupDialogParams params = new GroupDialogParams(group -> { });

    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private TextField inviteCodeField;
    @FXML private HBox groupPreviewBox;
    @FXML private Label groupInitialsLabel;
    @FXML private Label groupNameLabel;
    @FXML private Label groupDetailsLabel;
    @FXML private Button cancelButton;
    @FXML private Button joinButton;

    public JoinGroupController(CoreApi core, BackgroundTasks tasks) {
        this.core = Objects.requireNonNull(core);
        this.tasks = Objects.requireNonNull(tasks);
    }

    @FXML
    private void initialize() {
        inviteCodeField.textProperty().addListener((obs, old, text) -> {
            inviteCodeField.getStyleClass().remove(FIELD_INVALID);
            AlertBox.hide(errorBox);
        });
    }

    @Override
    public Class<GroupDialogParams> parametersType() {
        return GroupDialogParams.class;
    }

    @Override
    public void setParameters(GroupDialogParams parameters) {
        this.params = Objects.requireNonNull(parameters);
    }

    @FXML
    private void onJoinGroup() {
        String code = extractCode(inviteCodeField.getText());
        if (code.isEmpty() || code.length() > CODE_MAX) {
            inviteCodeField.getStyleClass().add(FIELD_INVALID);
            showError("Escribe el código de invitación.");
            inviteCodeField.requestFocus();
            return;
        }
        BusyButton.setBusy(joinButton, true, "Uniéndote…");
        tasks.run(() -> core.joinGroup(code), group -> {
            joinButton.getScene().getWindow().hide();
            params.onGroup().accept(group);
        }, error -> {
            BusyButton.setBusy(joinButton, false, null);
            inviteCodeField.getStyleClass().add(FIELD_INVALID);
            showError(CoreMessages.forError(error));
        });
    }

    static String extractCode(String text) {
        if (text == null) {
            return "";
        }
        String value = text.strip().replaceAll("[/?#]+$", "");
        int slash = value.lastIndexOf('/');
        return slash >= 0 ? value.substring(slash + 1) : value;
    }

    @FXML
    private void onCancel() {
        cancelButton.getScene().getWindow().hide();
    }

    private void showError(String message) {
        errorLabel.setText(message);
        AlertBox.setShown(errorBox, true);
    }
}
