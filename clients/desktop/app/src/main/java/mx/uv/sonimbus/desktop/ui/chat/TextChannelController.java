package mx.uv.sonimbus.desktop.ui.chat;

import java.time.ZoneId;
import java.time.format.DateTimeFormatter;
import java.util.List;
import java.util.Objects;
import java.util.function.Consumer;
import javafx.application.Platform;
import javafx.collections.ListChangeListener;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ListCell;
import javafx.scene.control.ContextMenu;
import javafx.scene.control.ListView;
import javafx.scene.control.MenuItem;
import javafx.scene.control.TextField;
import javafx.scene.control.TextFormatter;
import javafx.scene.layout.HBox;
import javafx.scene.layout.VBox;
import mx.uv.sonimbus.desktop.data.core.CoreApi;
import mx.uv.sonimbus.desktop.data.core.Message;
import mx.uv.sonimbus.desktop.data.realtime.ChannelConnection;
import mx.uv.sonimbus.desktop.data.realtime.RealtimeClient;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxMessage;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxService;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxStatus;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.CoreMessages;
import mx.uv.sonimbus.desktop.ui.common.Dialogs;
import mx.uv.sonimbus.desktop.ui.navigation.Parameterized;

public class TextChannelController implements Parameterized<TextChannelParams> {

    static final int PAGE_SIZE = 50;
    static final int CONTENT_MAX = 4000;
    private static final DateTimeFormatter TIME = DateTimeFormatter.ofPattern("d MMM, HH:mm")
            .withZone(ZoneId.systemDefault());

    private final CoreApi core;
    private final RealtimeClient realtime;
    private final BackgroundTasks tasks;
    private final OutboxService outbox;
    private ChatTimeline timeline;
    private TextChannelParams params;
    private ChannelConnection connection;
    private boolean joinedBefore;
    private boolean closed;

    @FXML private ListView<ChatEntry> messagesList;
    @FXML private Label emptyMessagesLabel;

    @FXML private HBox replyBox;
    @FXML private Label replyAuthorLabel;
    @FXML private Label replyPreviewLabel;
    @FXML private Button cancelReplyButton;

    @FXML private Label messageLabel;
    @FXML private TextField messageField;
    @FXML private Button attachButton;
    @FXML private Button sendButton;

    public TextChannelController(CoreApi core, RealtimeClient realtime, BackgroundTasks tasks,
            OutboxService outbox) {
        this.core = Objects.requireNonNull(core);
        this.realtime = Objects.requireNonNull(realtime);
        this.tasks = Objects.requireNonNull(tasks);
        this.outbox = Objects.requireNonNull(outbox);
    }

    @FXML
    private void initialize() {
        timeline = new ChatTimeline(messagesList.getItems());
        messagesList.setCellFactory(list -> new MessageCell(this::discard));
        messagesList.getItems().addListener((ListChangeListener<ChatEntry>) change ->
                showEmptyState());
        messageField.setTextFormatter(new TextFormatter<String>(change ->
                change.getControlNewText().length() <= CONTENT_MAX ? change : null));
        messagesList.sceneProperty().addListener((obs, old, scene) -> {
            if (old != null && scene == null) {
                close();
            }
        });
        attachButton.setDisable(true);
    }

    @Override
    public Class<TextChannelParams> parametersType() {
        return TextChannelParams.class;
    }

    @Override
    public void setParameters(TextChannelParams parameters) {
        this.params = Objects.requireNonNull(parameters);
        setStatus(null);
        loadUnsent();
        loadHistory();
        connection = realtime.channel(params.channel().id(), new ConnectionListener());
        connection.connect();
    }

    private void loadHistory() {
        String channelId = params.channel().id();
        tasks.run(() -> core.listMessages(channelId, null, PAGE_SIZE), page -> {
            if (closed) {
                return;
            }
            ChatTimeline.oldestFirst(page.data()).forEach(this::accept);
            scrollToEnd();
            showEmptyState();
        }, error -> {
            if (!closed) {
                Dialogs.showError(messagesList, CoreMessages.forError(error));
            }
        });
    }

    private void loadUnsent() {
        String channelId = params.channel().id();
        tasks.run(() -> outbox.unsent(channelId), this::showUnsent, error -> { });
    }

    private void showUnsent(List<OutboxMessage> unsent) {
        if (closed) {
            return;
        }
        for (OutboxMessage queued : unsent) {
            if (timeline.hasEntry(queued.clientMessageId())) {
                tasks.run(() -> {
                    outbox.confirm(queued.clientMessageId());
                    return null;
                }, ignored -> { }, error -> { });
                continue;
            }
            ChatEntry entry = queued.status() == OutboxStatus.FAILED
                    ? ChatEntry.failed(queued.clientMessageId(), queued.content())
                    : ChatEntry.pending(queued.clientMessageId(), queued.content());
            timeline.addPending(entry);
            if (entry.isRetryable() && joinedBefore) {
                send(entry);
            }
        }
        scrollToEnd();
    }

    private void accept(Message message) {
        boolean wasPending = timeline.hasUnconfirmed(message.clientMessageId());
        boolean added = timeline.accept(message);
        if (wasPending) {
            tasks.run(() -> {
                outbox.confirm(message.clientMessageId());
                return null;
            }, ignored -> { }, error -> { });
        }
        if (added) {
            scrollToEnd();
        }
    }

    @FXML
    private void onSend() {
        String text = messageField.getText() == null ? "" : messageField.getText().strip();
        if (text.isEmpty() || params == null) {
            return;
        }
        String channelId = params.channel().id();
        messageField.clear();
        tasks.run(() -> outbox.enqueue(channelId, text), queued -> {
            if (closed) {
                return;
            }
            ChatEntry entry = ChatEntry.pending(queued.clientMessageId(), queued.content());
            timeline.addPending(entry);
            scrollToEnd();
            send(entry);
        }, error -> {
            if (!closed) {
                messageField.setText(text);
                Dialogs.showError(messagesList, CoreMessages.MESSAGE_NOT_QUEUED);
            }
        });
    }

    private void send(ChatEntry entry) {
        connection.send(entry.clientMessageId(), entry.text()).whenComplete((message, error) ->
                Platform.runLater(() -> {
                    if (closed) {
                        return;
                    }
                    if (error == null) {
                        accept(message);
                        return;
                    }
                    String notice = CoreMessages.forSend(error);
                    if (notice.equals(CoreMessages.MESSAGE_NOT_SENT)) {
                        return;
                    }
                    timeline.markFailed(entry.clientMessageId());
                    tasks.run(() -> {
                        outbox.markFailed(entry.clientMessageId());
                        return null;
                    }, ignored -> { }, failure -> { });
                    Dialogs.showError(messagesList, notice);
                }));
    }

    private void discard(ChatEntry entry) {
        timeline.removePending(entry.clientMessageId());
        tasks.run(() -> {
            outbox.discard(entry.clientMessageId());
            return null;
        }, ignored -> { }, error -> { });
    }

    private void onJoined() {
        setStatus(null);
        if (joinedBefore) {
            loadHistory();
        }
        joinedBefore = true;
        for (ChatEntry entry : timeline.pending()) {
            send(entry);
        }
    }

    @FXML
    private void onAttachFile() {
    }

    @FXML
    private void onCancelReply() {
    }

    private void close() {
        closed = true;
        if (connection != null) {
            connection.close();
        }
    }

    private void setStatus(String status) {
        String base = params == null ? "Mensaje para el canal" : "Mensaje para #" + params.channel().name();
        messageLabel.setText(status == null ? base : base + " · " + status);
    }

    private void showEmptyState() {
        boolean empty = messagesList.getItems().isEmpty();
        emptyMessagesLabel.setVisible(empty);
        emptyMessagesLabel.setManaged(empty);
    }

    private void scrollToEnd() {
        if (!messagesList.getItems().isEmpty()) {
            messagesList.scrollTo(messagesList.getItems().size() - 1);
        }
    }

    private final class ConnectionListener implements ChannelConnection.Listener {
        @Override
        public void onJoined() {
            Platform.runLater(() -> {
                if (!closed) {
                    TextChannelController.this.onJoined();
                }
            });
        }

        @Override
        public void onMessage(Message message) {
            Platform.runLater(() -> {
                if (!closed) {
                    accept(message);
                }
            });
        }

        @Override
        public void onDisconnected() {
            Platform.runLater(() -> {
                if (!closed) {
                    setStatus("reconectando…");
                }
            });
        }

        @Override
        public void onAccessDenied() {
            Platform.runLater(() -> {
                if (!closed) {
                    sendButton.setDisable(true);
                    setStatus("sin acceso");
                    Dialogs.showError(messagesList, CoreMessages.NOT_FOUND);
                }
            });
        }
    }

    private static final class MessageCell extends ListCell<ChatEntry> {
        private final Label author = new Label();
        private final Label time = new Label();
        private final Label text = new Label();
        private final ContextMenu menu = new ContextMenu();
        private final VBox box = new VBox(2, new HBox(8, author, time), text);

        MessageCell(Consumer<ChatEntry> onDiscard) {
            MenuItem discard = new MenuItem("Descartar mensaje");
            discard.setOnAction(event -> {
                if (getItem() != null) {
                    onDiscard.accept(getItem());
                }
            });
            menu.getItems().add(discard);
            author.getStyleClass().add("message-author");
            time.getStyleClass().add("message-time");
            text.setWrapText(true);
            text.maxWidthProperty().bind(widthProperty().subtract(48));
        }

        @Override
        protected void updateItem(ChatEntry entry, boolean empty) {
            super.updateItem(entry, empty);
            setText(null);
            if (empty || entry == null) {
                setGraphic(null);
                setAccessibleText(null);
                setContextMenu(null);
                return;
            }
            text.getStyleClass().removeAll("message-text", "message-pending", "message-deleted");
            if (entry.isPending()) {
                author.setText("Tú");
                boolean failed = entry.state() == SendState.FAILED;
                time.setText(failed ? "No enviado" : "Pendiente · se enviará al reconectar");
                setContextMenu(failed ? menu : null);
                text.getStyleClass().add("message-pending");
            } else {
                Message message = entry.message();
                setContextMenu(null);
                author.setText(authorName(message));
                time.setText(TIME.format(message.createdInstant()));
                text.getStyleClass().add(message.deleted() ? "message-deleted" : "message-text");
            }
            text.setText(entry.isPending() || !entry.message().deleted() ? entry.text() : "Mensaje eliminado");
            setGraphic(box);
            setAccessibleText(author.getText() + ", " + time.getText() + ": " + text.getText());
        }

        private static String authorName(Message message) {
            String name = message.author() == null ? null : message.author().displayName();
            return name == null || name.isBlank() ? "Usuario" : name;
        }
    }
}
