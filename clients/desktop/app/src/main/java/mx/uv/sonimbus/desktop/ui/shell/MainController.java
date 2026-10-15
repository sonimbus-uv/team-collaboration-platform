package mx.uv.sonimbus.desktop.ui.shell;

import javafx.fxml.FXML;
import javafx.scene.Parent;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ListView;
import javafx.scene.control.MenuButton;
import javafx.scene.control.MenuItem;
import javafx.scene.control.Separator;
import javafx.scene.image.ImageView;
import javafx.scene.layout.StackPane;
import javafx.scene.layout.VBox;
import mx.uv.sonimbus.desktop.ui.navigation.ContentHost;
import mx.uv.sonimbus.desktop.ui.navigation.Navigable;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

public class MainController implements Navigable, ContentHost {

    private static final String SELECTED = "selected";

    private ViewManager viewManager;

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
        viewManager.showDialog(View.CREATE_GROUP, "Crear grupo");
    }

    @FXML
    private void onJoinGroup() {
        viewManager.showDialog(View.JOIN_GROUP, "Unirse a un grupo");
    }

    @FXML
    private void onOpenGroupSettings() {
        showSection(View.GROUP_SETTINGS, "Configuración del grupo", "", null);
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
        showSection(View.SHARED_FILES, "Archivos compartidos",
                "Archivos enviados en los canales del grupo", sharedFilesButton);
    }

    @FXML
    private void onShowProjects() {
        showSection(View.PROJECTS, "Proyectos y tareas", "", projectsButton);
    }

    @FXML
    private void onShowReports() {
        showSection(View.REPORTS, "Reportes", "", reportsButton);
    }

    @FXML
    private void onShowNotifications() {
        showSection(View.NOTIFICATIONS, "Notificaciones", "", null);
    }

    @FXML
    private void onShowProfile() {
        showSection(View.PROFILE, "Mi perfil", "", null);
    }

    @Override
    public void setViewManager(ViewManager viewManager) {
        this.viewManager = viewManager;
    }

    @Override
    public void setContent(Parent content) {
        contentPane.getChildren().setAll(content);
    }

    @Override
    public void clearContent() {
        contentPane.getChildren().setAll(emptyStateLabel);
        setSectionHeader("", "");
        selectNavItem(null);
    }

    private void showSection(View view, String title, String subtitle, Button navItem) {
        viewManager.showContent(view);
        setSectionHeader(title, subtitle);
        selectNavItem(navItem);
    }

    private void setSectionHeader(String title, String subtitle) {
        sectionTitleLabel.setText(title);
        sectionSubtitleLabel.setText(subtitle);
        boolean hasSubtitle = !subtitle.isEmpty();
        sectionTitleSeparator.setVisible(hasSubtitle);
        sectionTitleSeparator.setManaged(hasSubtitle);
    }

    /** Marca en la columna de canales la sección abierta; {@code null} las desmarca todas. */
    private void selectNavItem(Button selected) {
        for (Button navItem : new Button[] {sharedFilesButton, projectsButton, reportsButton}) {
            navItem.getStyleClass().remove(SELECTED);
        }
        if (selected != null) {
            selected.getStyleClass().add(SELECTED);
        }
    }
}
