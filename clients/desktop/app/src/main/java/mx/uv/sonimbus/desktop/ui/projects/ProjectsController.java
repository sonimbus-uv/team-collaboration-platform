package mx.uv.sonimbus.desktop.ui.projects;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.ComboBox;
import javafx.scene.control.Label;
import javafx.scene.control.ListView;
import javafx.scene.layout.VBox;

public class ProjectsController {

    // Lista de proyectos
    @FXML private Label projectsCountLabel;
    @FXML private ListView<Object> projectsList;         // TODO: modelo de proyecto
    @FXML private Button newProjectButton;

    // Tablero
    @FXML private VBox boardBox;
    @FXML private Label projectNameLabel;
    @FXML private Label projectSummaryLabel;
    @FXML private ComboBox<Object> assigneeFilterCombo;  // TODO: modelo de miembro
    @FXML private Button newTaskButton;
    @FXML private Label pendingCountLabel;
    @FXML private ListView<Object> pendingTasksList;     // TODO: modelo de tarea
    @FXML private Button addPendingTaskButton;
    @FXML private Label inProgressCountLabel;
    @FXML private ListView<Object> inProgressTasksList;  // TODO: modelo de tarea
    @FXML private Button addInProgressTaskButton;
    @FXML private Label completedCountLabel;
    @FXML private ListView<Object> completedTasksList;   // TODO: modelo de tarea
    @FXML private Label noProjectLabel;

    @FXML
    private void initialize() {
        // TODO CU-25: cell factory de projectsList (nombre + "N de M completadas" o
        //             "Sin tareas"); actualizar projectsCountLabel ("PROYECTOS (N)");
        //             newProjectButton solo para el administrador
        // TODO CU-25: al seleccionar un proyecto, llenar projectNameLabel, projectSummaryLabel
        //             ("Entrega final: dd/mm/aaaa · N de M completadas") y las tres listas;
        //             sin proyecto seleccionado ocultar boardBox y mostrar noProjectLabel
        // TODO CU-26: cell factory de las tres listas con una tarjeta por tarea ("task-card"):
        //             título ("task-title"; + "task-done" en completadas), responsable con
        //             avatar o "Sin asignar" ("task-meta") y fecha límite ("Vence dd/mm";
        //             vencida: "status-chip" + "status-danger" con icono y texto
        //             "Vencida · dd/mm"; completada: icono de palomita + fecha)
        // TODO CU-26: doble clic o Enter sobre una tarea -> abrir projects/task-detail.fxml en
        //             un diálogo modal
        // TODO CU-28: actualizar pendingCountLabel, inProgressCountLabel y completedCountLabel
        //             (texto y accessibleText) al cambiar el estado de una tarea
        // TODO CU-27: llenar assigneeFilterCombo con "Todos", "Yo" y los miembros del grupo
    }

    @FXML
    private void onNewProject() {
        // TODO CU-25: pedir nombre (y fecha de entrega) del proyecto y crearlo
    }

    @FXML
    private void onNewTask() {
        // TODO CU-26: abrir projects/task-detail.fxml en un diálogo modal para una tarea nueva
        //             del proyecto seleccionado
    }

    @FXML
    private void onAddPendingTask() {
        // TODO CU-26: igual que onNewTask, con el estado inicial "Pendiente"
    }

    @FXML
    private void onAddInProgressTask() {
        // TODO CU-26: igual que onNewTask, con el estado inicial "En progreso"
    }

    @FXML
    private void onAssigneeFilterChanged() {
        // TODO CU-26: filtrar las tareas del tablero por el responsable elegido
    }
}
