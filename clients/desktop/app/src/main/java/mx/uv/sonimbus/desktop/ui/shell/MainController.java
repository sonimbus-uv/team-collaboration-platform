package mx.uv.sonimbus.desktop.ui.shell;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ListView;
import javafx.scene.control.MenuButton;
import javafx.scene.control.MenuItem;
import javafx.scene.control.Separator;
import javafx.scene.image.ImageView;
import javafx.scene.layout.StackPane;
import javafx.scene.layout.VBox;

public class MainController {

    // Columna de grupos
    @FXML private ListView<Object> groupsList;           // TODO: cambiar Object por el modelo de grupo
    @FXML private MenuButton addGroupMenuButton;
    @FXML private MenuItem createGroupMenuItem;
    @FXML private MenuItem joinGroupMenuItem;

    // Columna de canales
    @FXML private VBox channelsColumn;
    @FXML private Label groupNameLabel;
    @FXML private Button groupSettingsButton;
    @FXML private Button createTextChannelButton;
    @FXML private ListView<Object> textChannelsList;     // TODO: modelo de canal
    @FXML private Button createVoiceChannelButton;
    @FXML private ListView<Object> voiceChannelsList;    // TODO: modelo de canal de voz
    @FXML private Button sharedFilesButton;
    @FXML private Button projectsButton;
    @FXML private Button reportsButton;

    // Barra superior
    @FXML private Label sectionTitleLabel;
    @FXML private Separator sectionTitleSeparator;
    @FXML private Label sectionSubtitleLabel;
    @FXML private Button notificationsButton;
    @FXML private Label notificationsBadgeLabel;
    @FXML private Button profileButton;
    @FXML private Label profileInitialsLabel;
    @FXML private ImageView profileAvatarView;

    // Contenido
    @FXML private StackPane contentPane;
    @FXML private Label emptyStateLabel;

    @FXML
    private void initialize() {
        // TODO: cell factories de groupsList (iniciales/imagen, indicador de seleccionado,
        //       insignia de no leídos), textChannelsList ("# nombre", insignia) y
        //       voiceChannelsList (icono de altavoz + participantes conectados)
        // TODO: al seleccionar grupo -> cargar canales y groupNameLabel
        // TODO CU-16: al seleccionar canal de texto -> cargar chat/text-channel.fxml en contentPane
        //             y actualizar sectionTitleLabel/sectionSubtitleLabel
        // TODO CU-22: al seleccionar canal de voz -> cargar voice/voice-channel.fxml en contentPane
        // TODO CU-31: actualizar notificationsBadgeLabel y accessibleText
        //             ("Notificaciones, N sin leer") de notificationsButton
        // TODO CU-07: mostrar avatar del perfil (circle clip en profileAvatarView) o iniciales
    }

    @FXML
    private void onCreateGroup() {
        // TODO CU-08: abrir groups/create-group.fxml en un diálogo modal
    }

    @FXML
    private void onJoinGroup() {
        // TODO CU-11: abrir groups/join-group.fxml en un diálogo modal
    }

    @FXML
    private void onOpenGroupSettings() {
        // TODO CU-09: cargar groups/group-settings.fxml en contentPane
    }

    @FXML
    private void onCreateTextChannel() {
        // TODO CU-13: crear canal de texto en el grupo seleccionado
    }

    @FXML
    private void onCreateVoiceChannel() {
        // TODO CU-13: crear canal de voz en el grupo seleccionado
    }

    @FXML
    private void onShowSharedFiles() {
        // TODO CU-24: cargar files/shared-files.fxml en contentPane
    }

    @FXML
    private void onShowProjects() {
        // TODO CU-25: cargar projects/projects.fxml en contentPane
    }

    @FXML
    private void onShowReports() {
        // TODO CU-29: cargar reports/reports.fxml en contentPane
    }

    @FXML
    private void onShowNotifications() {
        // TODO CU-31: cargar notifications/notifications.fxml en contentPane
    }

    @FXML
    private void onShowProfile() {
        // TODO CU-07: cargar profile/profile.fxml en contentPane
    }
}
