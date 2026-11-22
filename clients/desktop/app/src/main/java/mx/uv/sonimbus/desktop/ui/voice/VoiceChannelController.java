package mx.uv.sonimbus.desktop.ui.voice;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ToggleButton;
import javafx.scene.layout.HBox;
import javafx.scene.layout.TilePane;
import javafx.scene.layout.VBox;
import javafx.scene.shape.SVGPath;

public class VoiceChannelController {

    @FXML private VBox noticesBox;
    @FXML private HBox unavailableBox;
    @FXML private Label unavailableLabel;
    @FXML private Button retryButton;
    @FXML private HBox listenOnlyBox;
    @FXML private Label listenOnlyLabel;

    @FXML private Label participantsCountLabel;
    @FXML private TilePane participantsPane;

    @FXML private ToggleButton muteToggle;
    @FXML private SVGPath muteIcon;
    @FXML private Button leaveButton;

    @FXML
    private void initialize() {
    }

    @FXML
    private void onToggleMute() {
    }

    @FXML
    private void onLeave() {
    }

    @FXML
    private void onRetry() {
    }
}
