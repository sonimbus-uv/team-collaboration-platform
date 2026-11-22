package mx.uv.sonimbus.desktop.ui.notifications;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ListView;
import javafx.scene.control.ToggleButton;
import javafx.scene.control.ToggleGroup;

public class NotificationsController {

    @FXML private ToggleGroup filterGroup;
    @FXML private ToggleButton allFilterToggle;
    @FXML private ToggleButton unreadFilterToggle;
    @FXML private Button markAllReadButton;
    @FXML private ListView<Object> notificationsList;
    @FXML private Label emptyStateLabel;

    @FXML
    private void initialize() {
    }

    @FXML
    private void onFilterChanged() {
    }

    @FXML
    private void onMarkAllRead() {
    }
}
