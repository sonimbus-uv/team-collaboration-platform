package mx.uv.sonimbus.desktop.ui.groups;

import java.util.Objects;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.TextArea;
import javafx.scene.control.TextField;
import javafx.scene.control.TextFormatter;
import javafx.scene.image.ImageView;
import javafx.scene.layout.HBox;
import mx.uv.sonimbus.desktop.data.core.CoreApi;
import mx.uv.sonimbus.desktop.data.core.Group;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.common.CoreMessages;
import mx.uv.sonimbus.desktop.ui.common.Initials;
import mx.uv.sonimbus.desktop.ui.navigation.Parameterized;

public class CreateGroupController implements Parameterized<GroupDialogParams> {

    private static final String FIELD_INVALID = "field-invalid";
    static final int NAME_MIN = 2;
    static final int NAME_MAX = 100;
    static final int DESCRIPTION_MAX = 200;
    static final String DEFAULT_CHANNEL = "general";

    private final CoreApi core;
    private final BackgroundTasks tasks;
    private GroupDialogParams params = new GroupDialogParams(group -> { });

    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private TextField nameField;
    @FXML private TextArea descriptionArea;
    @FXML private Label descriptionCountLabel;
    @FXML private Label imageInitialsLabel;
    @FXML private ImageView groupImageView;
    @FXML private Button chooseImageButton;
    @FXML private Button removeImageButton;
    @FXML private Button cancelButton;
    @FXML private Button createButton;

    public CreateGroupController(CoreApi core, BackgroundTasks tasks) {
        this.core = Objects.requireNonNull(core);
        this.tasks = Objects.requireNonNull(tasks);
    }

    @FXML
    private void initialize() {
        descriptionArea.setTextFormatter(new TextFormatter<String>(change ->
                change.getControlNewText().length() <= DESCRIPTION_MAX ? change : null));
        descriptionArea.textProperty().addListener((obs, old, text) ->
                descriptionCountLabel.setText(text.length() + " / " + DESCRIPTION_MAX));
        nameField.textProperty().addListener((obs, old, text) -> {
            imageInitialsLabel.setText(Initials.of(text));
            nameField.getStyleClass().remove(FIELD_INVALID);
            AlertBox.hide(errorBox);
        });
        chooseImageButton.setDisable(true);
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
    private void onChooseImage() {
    }

    @FXML
    private void onRemoveImage() {
    }

    @FXML
    private void onCreateGroup() {
        String name = nameField.getText() == null ? "" : nameField.getText().strip();
        if (name.length() < NAME_MIN || name.length() > NAME_MAX) {
            nameField.getStyleClass().add(FIELD_INVALID);
            showError("Escribe un nombre de " + NAME_MIN + " a " + NAME_MAX + " caracteres.");
            nameField.requestFocus();
            return;
        }
        String description = descriptionArea.getText().isBlank() ? null : descriptionArea.getText().strip();
        BusyButton.setBusy(createButton, true, "Creando…");
        tasks.run(() -> createWithDefaultChannel(name, description), group -> {
            createButton.getScene().getWindow().hide();
            params.onGroup().accept(group);
        }, error -> {
            BusyButton.setBusy(createButton, false, null);
            showError(CoreMessages.forError(error));
        });
    }

    private Group createWithDefaultChannel(String name, String description) throws Exception {
        Group group = core.createGroup(name, description);
        try {
            core.createChannel(group.id(), DEFAULT_CHANNEL, "text");
        } catch (Exception e) {
        }
        return group;
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
