package mx.uv.sonimbus.desktop.ui.shell;

import java.util.List;
import java.util.Locale;
import java.util.Objects;
import java.util.Optional;
import java.util.regex.Pattern;
import javafx.fxml.FXML;
import javafx.scene.Parent;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ListCell;
import javafx.scene.control.ListView;
import javafx.scene.control.MenuButton;
import javafx.scene.control.MenuItem;
import javafx.scene.control.Separator;
import javafx.scene.control.Tooltip;
import javafx.scene.image.ImageView;
import javafx.scene.layout.StackPane;
import javafx.scene.layout.VBox;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.core.Channel;
import mx.uv.sonimbus.desktop.data.core.CoreApi;
import mx.uv.sonimbus.desktop.data.core.Group;
import mx.uv.sonimbus.desktop.data.core.Profile;
import mx.uv.sonimbus.desktop.ui.chat.TextChannelParams;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.CoreMessages;
import mx.uv.sonimbus.desktop.ui.common.Dialogs;
import mx.uv.sonimbus.desktop.ui.common.Initials;
import mx.uv.sonimbus.desktop.ui.groups.GroupDialogParams;
import mx.uv.sonimbus.desktop.ui.navigation.ContentHost;
import mx.uv.sonimbus.desktop.ui.navigation.Navigable;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

/**
 * Shell: grupos del usuario, canales del grupo elegido y el canal abierto en el centro. Al
 * entrar pide el nombre visible si el usuario todavía no lo guardó (D-29).
 */
public class MainController implements Navigable, ContentHost {

    private static final String SELECTED = "selected";
    /** {@code CreateChannel.name} de core.yaml. */
    static final Pattern CHANNEL_NAME = Pattern.compile("^[a-z0-9-]{1,80}$");
    static final int DISPLAY_NAME_MAX = 100;

    private final CoreApi core;
    private final BackgroundTasks tasks;
    private ViewManager viewManager;
    private Group currentGroup;

    // Columna de grupos
    @FXML private ListView<Group> groupsList;
    @FXML private MenuButton addGroupMenuButton;
    @FXML private MenuItem createGroupMenuItem;
    @FXML private MenuItem joinGroupMenuItem;

    // Columna de canales
    @FXML private VBox channelsColumn;
    @FXML private Label groupNameLabel;
    @FXML private Button inviteButton;
    @FXML private Button groupSettingsButton;
    @FXML private Button createTextChannelButton;
    @FXML private ListView<Channel> textChannelsList;
    @FXML private Button createVoiceChannelButton;
    @FXML private ListView<Channel> voiceChannelsList;    // TODO CU-22: participantes conectados
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

    public MainController(CoreApi core, BackgroundTasks tasks) {
        this.core = Objects.requireNonNull(core);
        this.tasks = Objects.requireNonNull(tasks);
    }

    @FXML
    private void initialize() {
        groupsList.setCellFactory(list -> new GroupCell());
        textChannelsList.setCellFactory(list -> new ChannelCell("# "));
        voiceChannelsList.setCellFactory(list -> new ChannelCell("🔊 "));
        groupsList.getSelectionModel().selectedItemProperty().addListener((obs, old, group) -> {
            if (group != null && (currentGroup == null || !group.id().equals(currentGroup.id()))) {
                selectGroup(group, null);
            }
        });
        textChannelsList.getSelectionModel().selectedItemProperty().addListener((obs, old, channel) -> {
            if (channel != null) {
                openTextChannel(channel);
            }
        });
        showGroupActions(null);
        // TODO CU-31: actualizar notificationsBadgeLabel y accessibleText
        //             ("Notificaciones, N sin leer") de notificationsButton
        // TODO CU-07: mostrar avatar del perfil (circle clip en profileAvatarView)
        tasks.run(core::myProfile, profile -> {
            profile.ifPresentOrElse(this::showProfile, this::askDisplayName);
            loadGroups(null);
        }, this::showError);
    }

    // --- Perfil (D-29) ---

    private void askDisplayName() {
        Dialogs.askText(groupsList, "Tu nombre visible",
                        "¿Cómo quieres que te vean los demás en los grupos?", "")
                .filter(name -> !name.isEmpty() && name.length() <= DISPLAY_NAME_MAX)
                .ifPresent(name -> tasks.run(() -> core.saveDisplayName(name), this::showProfile, this::showError));
    }

    private void showProfile(Profile profile) {
        profileInitialsLabel.setText(Initials.of(profile.displayName()));
        profileButton.setAccessibleText("Mi perfil, " + profile.displayName());
        Tooltip.install(profileButton, new Tooltip(profile.displayName()));
    }

    // --- Grupos ---

    private void loadGroups(String selectId) {
        tasks.run(core::listGroups, groups -> {
            groupsList.getItems().setAll(groups);
            groups.stream()
                    .filter(g -> selectId == null ? currentGroup == null : g.id().equals(selectId))
                    .findFirst()
                    .or(() -> groups.stream().findFirst())
                    .ifPresent(g -> {
                        currentGroup = null;
                        groupsList.getSelectionModel().select(g);
                    });
        }, this::showError);
    }

    private void selectGroup(Group group, String selectChannelId) {
        currentGroup = group;
        groupNameLabel.setText(group.name());
        showGroupActions(group);
        clearContent();
        loadChannels(group, selectChannelId);
    }

    private void loadChannels(Group group, String selectChannelId) {
        tasks.run(() -> core.listChannels(group.id()), channels -> {
            if (currentGroup == null || !currentGroup.id().equals(group.id())) {
                return;
            }
            List<Channel> text = channels.stream().filter(Channel::isText).toList();
            textChannelsList.getItems().setAll(text);
            voiceChannelsList.getItems().setAll(channels.stream().filter(c -> !c.isText()).toList());
            text.stream()
                    .filter(c -> selectChannelId == null ? "general".equals(c.name()) : c.id().equals(selectChannelId))
                    .findFirst()
                    .or(() -> selectChannelId == null ? text.stream().findFirst() : Optional.empty())
                    .ifPresent(c -> textChannelsList.getSelectionModel().select(c));
        }, this::showError);
    }

    private void showGroupActions(Group group) {
        boolean selected = group != null;
        inviteButton.setDisable(!selected || !group.canInvite());
        createTextChannelButton.setDisable(!selected || !group.isAdmin());
        createVoiceChannelButton.setDisable(!selected || !group.isAdmin());
        groupSettingsButton.setDisable(!selected);
        if (!selected) {
            groupNameLabel.setText("Selecciona un grupo");
            textChannelsList.getItems().clear();
            voiceChannelsList.getItems().clear();
        }
    }

    @FXML
    private void onCreateGroup() {
        viewManager.showDialog(View.CREATE_GROUP, "Crear grupo", new GroupDialogParams(this::groupAdded));
    }

    @FXML
    private void onJoinGroup() {
        viewManager.showDialog(View.JOIN_GROUP, "Unirse a un grupo", new GroupDialogParams(this::groupAdded));
    }

    private void groupAdded(Group group) {
        currentGroup = null;
        loadGroups(group.id());
    }

    /** CU-10: invitación con los valores por omisión (7 días, usos ilimitados). */
    @FXML
    private void onInvite() {
        Group group = currentGroup;
        if (group == null) {
            return;
        }
        inviteButton.setDisable(true);
        tasks.run(() -> core.createInvitation(group.id()), invitation -> {
            inviteButton.setDisable(false);
            Dialogs.showInvitationCode(groupsList, group.name(), invitation.code());
        }, error -> {
            inviteButton.setDisable(false);
            showError(error);
        });
    }

    @FXML
    private void onOpenGroupSettings() {
        viewManager.showContent(View.GROUP_SETTINGS);
    }

    // --- Canales (CU-13) ---

    @FXML
    private void onCreateTextChannel() {
        createChannel("text");
    }

    @FXML
    private void onCreateVoiceChannel() {
        createChannel("voice");
    }

    private void createChannel(String type) {
        Group group = currentGroup;
        if (group == null) {
            return;
        }
        String kind = "text".equals(type) ? "de texto" : "de voz";
        Dialogs.askText(groupsList, "Crear canal " + kind,
                        "Nombre del canal: minúsculas, números y guiones", "")
                .map(name -> name.toLowerCase(Locale.ROOT).replace(' ', '-'))
                .ifPresent(name -> {
                    if (!CHANNEL_NAME.matcher(name).matches()) {
                        Dialogs.showError(groupsList, "Usa solo minúsculas, números y guiones (máximo 80).");
                        return;
                    }
                    tasks.run(() -> core.createChannel(group.id(), name, type),
                            channel -> loadChannels(group, channel.isText() ? channel.id() : null),
                            this::showError);
                });
    }

    private void openTextChannel(Channel channel) {
        Group group = currentGroup;
        if (group == null) {
            return;
        }
        viewManager.showContent(View.TEXT_CHANNEL, new TextChannelParams(group, channel));
        setSectionHeader("# " + channel.name(), group.name());
    }

    // --- Otras secciones ---

    @FXML
    private void onShowSharedFiles() {
        viewManager.showContent(View.SHARED_FILES);
    }

    @FXML
    private void onShowProjects() {
        viewManager.showContent(View.PROJECTS);
    }

    @FXML
    private void onShowReports() {
        viewManager.showContent(View.REPORTS);
    }

    @FXML
    private void onShowNotifications() {
        viewManager.showContent(View.NOTIFICATIONS);
    }

    @FXML
    private void onShowProfile() {
        viewManager.showContent(View.PROFILE);
    }

    @Override
    public void setViewManager(ViewManager viewManager) {
        this.viewManager = viewManager;
    }

    @Override
    public void setContent(View view, Parent content) {
        contentPane.getChildren().setAll(content);
        if (view != View.TEXT_CHANNEL) {
            textChannelsList.getSelectionModel().clearSelection();
        }
        switch (view) {
            case TEXT_CHANNEL -> showSectionHeader("Canal de texto", "", null);
            case VOICE_CHANNEL -> showSectionHeader("Canal de voz", "", null);
            case SHARED_FILES -> showSectionHeader("Archivos compartidos",
                    "Archivos enviados en los canales del grupo", sharedFilesButton);
            case GROUP_SETTINGS -> showSectionHeader("Configuración del grupo", "", null);
            case PROJECTS -> showSectionHeader("Proyectos y tareas", "", projectsButton);
            case REPORTS -> showSectionHeader("Reportes", "", reportsButton);
            case NOTIFICATIONS -> showSectionHeader("Notificaciones", "", null);
            case PROFILE -> showSectionHeader("Mi perfil", "", null);
            default -> showSectionHeader("", "", null);
        }
    }

    @Override
    public void clearContent() {
        contentPane.getChildren().setAll(emptyStateLabel);
        showSectionHeader("", "", null);
    }

    private void showError(Throwable error) {
        // Sesión vencida: Session ya lleva a la pantalla de inicio de sesión.
        if (error instanceof ApiException e && e.kind() == ApiErrorKind.UNAUTHORIZED) {
            return;
        }
        Dialogs.showError(groupsList, CoreMessages.forError(error));
    }

    private void showSectionHeader(String title, String subtitle, Button navItem) {
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

    /** Ficha redonda con las iniciales; el nombre va en el tooltip y en el texto accesible. */
    private static final class GroupCell extends ListCell<Group> {
        private final Label chip = new Label();

        GroupCell() {
            chip.getStyleClass().add("group-chip");
            selectedProperty().addListener((obs, old, selected) -> markSelected(selected));
        }

        @Override
        protected void updateItem(Group group, boolean empty) {
            super.updateItem(group, empty);
            setText(null);
            if (empty || group == null) {
                setGraphic(null);
                setTooltip(null);
                setAccessibleText(null);
                return;
            }
            chip.setText(Initials.of(group.name()));
            markSelected(isSelected());
            setGraphic(chip);
            setTooltip(new Tooltip(group.name()));
            setAccessibleText("Grupo " + group.name());
        }

        private void markSelected(boolean selected) {
            chip.getStyleClass().remove(SELECTED);
            if (selected) {
                chip.getStyleClass().add(SELECTED);
            }
        }
    }

    private static final class ChannelCell extends ListCell<Channel> {
        private final String prefix;

        ChannelCell(String prefix) {
            this.prefix = prefix;
        }

        @Override
        protected void updateItem(Channel channel, boolean empty) {
            super.updateItem(channel, empty);
            setText(empty || channel == null ? null : prefix + channel.name());
        }
    }
}
