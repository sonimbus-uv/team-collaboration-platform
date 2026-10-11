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
    @FXML private ListView<Object> notificationsList;    // TODO: modelo de notificación
    @FXML private Label emptyStateLabel;

    @FXML
    private void initialize() {
        // TODO CU-31: cell factory de notificationsList (más reciente primero, agrupadas por
        //             día con "day-separator"): indicador de leída/no leída con icono + texto
        //             accesible ("No leída"/"Leída"), título ("notification-title"; +
        //             "unread" en las no leídas), detalle ("notification-detail") y hora
        //             ("message-time"); la celda no leída lleva además la clase "unread"
        // TODO CU-31: al abrir una notificación, marcarla como leída y navegar a su origen
        //             (mensaje, tarea o grupo)
        // TODO CU-31: actualizar el texto de unreadFilterToggle ("No leídas (N)"), deshabilitar
        //             markAllReadButton si no hay no leídas y mostrar emptyStateLabel si la
        //             lista queda vacía
    }

    @FXML
    private void onFilterChanged() {
        // TODO CU-31: mostrar todas o solo las no leídas; impedir que filterGroup quede sin
        //             selección
    }

    @FXML
    private void onMarkAllRead() {
        // TODO CU-31: marcar todas las notificaciones como leídas y actualizar la insignia de
        //             la campana del shell
    }
}
