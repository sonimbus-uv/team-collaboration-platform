package mx.uv.sonimbus.desktop.ui.projects;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.ComboBox;
import javafx.scene.control.Label;
import javafx.scene.control.ListView;
import javafx.scene.layout.VBox;

public class ProjectsController {

    @FXML private Label projectsCountLabel;
    @FXML private ListView<Object> projectsList;
    @FXML private Button newProjectButton;

    @FXML private VBox boardBox;
    @FXML private Label projectNameLabel;
    @FXML private Label projectSummaryLabel;
    @FXML private ComboBox<Object> assigneeFilterCombo;
    @FXML private Button newTaskButton;
    @FXML private Label pendingCountLabel;
    @FXML private ListView<Object> pendingTasksList;
    @FXML private Button addPendingTaskButton;
    @FXML private Label inProgressCountLabel;
    @FXML private ListView<Object> inProgressTasksList;
    @FXML private Button addInProgressTaskButton;
    @FXML private Label completedCountLabel;
    @FXML private ListView<Object> completedTasksList;
    @FXML private Label noProjectLabel;

    @FXML
    private void initialize() {
    }

    @FXML
    private void onNewProject() {
    }

    @FXML
    private void onNewTask() {
    }

    @FXML
    private void onAddPendingTask() {
    }

    @FXML
    private void onAddInProgressTask() {
    }

    @FXML
    private void onAssigneeFilterChanged() {
    }
}
