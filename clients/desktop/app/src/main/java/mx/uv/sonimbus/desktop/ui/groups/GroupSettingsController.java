package mx.uv.sonimbus.desktop.ui.groups;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.ComboBox;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.ListView;
import javafx.scene.control.RadioButton;
import javafx.scene.control.Tab;
import javafx.scene.control.TabPane;
import javafx.scene.control.TableColumn;
import javafx.scene.control.TableView;
import javafx.scene.control.TextArea;
import javafx.scene.control.TextField;
import javafx.scene.control.ToggleGroup;
import javafx.scene.layout.BorderPane;
import javafx.scene.layout.HBox;
import javafx.scene.layout.StackPane;
import javafx.scene.layout.VBox;
import mx.uv.sonimbus.desktop.ui.navigation.Navigable;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

public class GroupSettingsController implements Navigable {

    private ViewManager viewManager;

    @FXML private BorderPane settingsPane;
    @FXML private Button backButton;
    @FXML private Button leaveGroupButton;
    @FXML private TabPane settingsTabs;

    @FXML private Tab generalTab;
    @FXML private HBox generalErrorBox;
    @FXML private Label generalErrorLabel;
    @FXML private TextField groupNameField;
    @FXML private TextArea groupDescriptionArea;
    @FXML private Label groupDescriptionCountLabel;
    @FXML private Button saveButton;
    @FXML private Button discardButton;
    @FXML private VBox dangerZoneBox;
    @FXML private Button archiveGroupButton;
    @FXML private Button deleteGroupButton;

    @FXML private Tab invitationsTab;
    @FXML private ComboBox<String> maxUsesCombo;
    @FXML private ComboBox<String> expirationCombo;
    @FXML private Button generateInviteButton;
    @FXML private TableView<Object> invitationsTable;
    @FXML private TableColumn<Object, Object> inviteCodeColumn;
    @FXML private TableColumn<Object, Object> inviteCreatorColumn;
    @FXML private TableColumn<Object, Object> inviteUsesColumn;
    @FXML private TableColumn<Object, Object> inviteExpirationColumn;
    @FXML private TableColumn<Object, Object> inviteStatusColumn;
    @FXML private TableColumn<Object, Object> inviteActionsColumn;

    @FXML private Tab channelsTab;
    @FXML private TextField newChannelNameField;
    @FXML private HBox channelTypeBox;
    @FXML private ToggleGroup channelTypeGroup;
    @FXML private RadioButton textTypeRadio;
    @FXML private RadioButton voiceTypeRadio;
    @FXML private Button createChannelButton;
    @FXML private Label textChannelsCountLabel;
    @FXML private ListView<Object> textChannelsList;
    @FXML private Label voiceChannelsCountLabel;
    @FXML private ListView<Object> voiceChannelsList;

    @FXML private Tab membersTab;
    @FXML private Label membersCountLabel;
    @FXML private TextField memberSearchField;
    @FXML private TableView<Object> membersTable;
    @FXML private TableColumn<Object, Object> memberColumn;
    @FXML private TableColumn<Object, Object> memberRoleColumn;
    @FXML private TableColumn<Object, Object> memberActionsColumn;
    @FXML private Label membersShownLabel;
    @FXML private Hyperlink showAllMembersLink;

    @FXML private Tab moderationTab;
    @FXML private Label blockedCountLabel;
    @FXML private TableView<Object> blockedTable;
    @FXML private TableColumn<Object, Object> blockedMemberColumn;
    @FXML private TableColumn<Object, Object> blockedDateColumn;
    @FXML private TableColumn<Object, Object> blockedByColumn;
    @FXML private TableColumn<Object, Object> blockedActionsColumn;

    @FXML private StackPane deleteConfirmOverlay;
    @FXML private Label deleteTitleLabel;
    @FXML private Label deleteDescriptionLabel;
    @FXML private Label deleteConfirmLabel;
    @FXML private TextField deleteConfirmField;
    @FXML private Button cancelDeleteButton;
    @FXML private Button confirmDeleteButton;

    @FXML
    private void initialize() {
    }

    @FXML
    private void onBackToGroup() {
        viewManager.closeContent();
    }

    @FXML
    private void onLeaveGroup() {
    }

    @FXML
    private void onSaveGeneral() {
    }

    @FXML
    private void onDiscardGeneral() {
    }

    @FXML
    private void onArchiveGroup() {
    }

    @FXML
    private void onDeleteGroup() {
    }

    @FXML
    private void onCancelDelete() {
    }

    @FXML
    private void onConfirmDelete() {
    }

    @FXML
    private void onGenerateInvite() {
    }

    @FXML
    private void onCreateChannel() {
    }

    @FXML
    private void onSearchMember() {
    }

    @FXML
    private void onShowAllMembers() {
    }


    private void copyInvite(Object invitation) {
    }

    private void revokeInvite(Object invitation) {
    }

    private void renameChannel(Object channel) {
    }

    private void deleteChannel(Object channel) {
    }

    private void changeMemberRole(Object member, String role) {
    }

    private void kickMember(Object member) {
    }

    private void blockMember(Object member) {
    }

    private void unblockMember(Object member) {
    }

    @Override
    public void setViewManager(ViewManager viewManager) {
        this.viewManager = viewManager;
    }
}
