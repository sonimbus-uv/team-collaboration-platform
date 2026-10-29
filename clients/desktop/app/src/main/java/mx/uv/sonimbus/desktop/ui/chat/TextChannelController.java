package mx.uv.sonimbus.desktop.ui.chat;

import java.time.ZoneId;
import java.time.format.DateTimeFormatter;
import java.util.Objects;
import java.util.UUID;
import javafx.application.Platform;
import javafx.collections.ListChangeListener;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ListCell;
import javafx.scene.control.ListView;
import javafx.scene.control.TextField;
import javafx.scene.control.TextFormatter;
import javafx.scene.layout.HBox;
import javafx.scene.layout.VBox;
import mx.uv.sonimbus.desktop.data.core.CoreApi;
import mx.uv.sonimbus.desktop.data.core.Message;
import mx.uv.sonimbus.desktop.data.realtime.ChannelConnection;
import mx.uv.sonimbus.desktop.data.realtime.RealtimeClient;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.CoreMessages;
import mx.uv.sonimbus.desktop.ui.common.Dialogs;
import mx.uv.sonimbus.desktop.ui.navigation.Parameterized;

/**
 * CU-16 y CU-20. Carga la página más reciente del historial por REST y recibe y envía mensajes
 * por el WebSocket de Core. Un mensaje propio aparece «Enviando…» hasta que Core lo confirma;
 * si se pierde la conexión, se reenvía al reconectar con el mismo {@code client_message_id}.
 * La conexión se cierra cuando la vista sale de la ventana.
 */
public class TextChannelController implements Parameterized<TextChannelParams> {

    static final int PAGE_SIZE = 50;
    static final int CONTENT_MAX = 4000;
    private static final DateTimeFormatter TIME = DateTimeFormatter.ofPattern("d MMM, HH:mm")
            .withZone(ZoneId.systemDefault());

    private final CoreApi core;
    private final RealtimeClient realtime;
    private final BackgroundTasks tasks;
    private ChatTimeline timeline;
    private TextChannelParams params;
    private ChannelConnection connection;
    private boolean joinedBefore;
    private boolean closed;

    // Historial
    @FXML private ListView<ChatEntry> messagesList;
    @FXML private Label emptyMessagesLabel;

    // Barra de respuesta
    @FXML private HBox replyBox;
    @FXML private Label replyAuthorLabel;
    @FXML private Label replyPreviewLabel;
    @FXML private Button cancelReplyButton;

    // Caja de escritura
    @FXML private Label messageLabel;
    @FXML private TextField messageField;
    @FXML private Button attachButton;
    @FXML private Button sendButton;

    public TextChannelController(CoreApi core, RealtimeClient realtime, BackgroundTasks tasks) {
        this.core = Objects.requireNonNull(core);
        this.realtime = Objects.requireNonNull(realtime);
        this.tasks = Objects.requireNonNull(tasks);
    }

    @FXML
    private void initialize() {
        timeline = new ChatTimeline(messagesList.getItems());
        messagesList.setCellFactory(list -> new MessageCell());
        messagesList.getItems().addListener((ListChangeListener<ChatEntry>) change ->
                showEmptyState());
        messageField.setTextFormatter(new TextFormatter<String>(change ->
                change.getControlNewText().length() <= CONTENT_MAX ? change : null));
        // Cierra el WebSocket cuando el shell reemplaza esta vista o se cierra la sesión.
        messagesList.sceneProperty().addListener((obs, old, scene) -> {
            if (old != null && scene == null) {
                close();
            }
        });
        // TODO CU-17, CU-18, CU-19: responder, reaccionar, editar y eliminar
        // TODO CU-20: cargar páginas anteriores al desplazarse hacia arriba
        // TODO CU-23: adjuntar archivos
        attachButton.setDisable(true);
        sendButton.setDisable(true);
    }

    @Override
    public Class<TextChannelParams> parametersType() {
        return TextChannelParams.class;
    }

    @Override
    public void setParameters(TextChannelParams parameters) {
        this.params = Objects.requireNonNull(parameters);
        setStatus(null);
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
            timeline.acceptAll(ChatTimeline.oldestFirst(page.data()));
            scrollToEnd();
            showEmptyState();
        }, error -> {
            if (!closed) {
                Dialogs.showError(messagesList, CoreMessages.forError(error));
            }
        });
    }

    @FXML
    private void onSend() {
        String text = messageField.getText() == null ? "" : messageField.getText().strip();
        if (text.isEmpty() || connection == null) {
            return;
        }
        ChatEntry entry = ChatEntry.pending(UUID.randomUUID().toString(), text);
        timeline.addPending(entry);
        messageField.clear();
        scrollToEnd();
        send(entry);
    }

    private void send(ChatEntry entry) {
        connection.send(entry.clientMessageId(), entry.text()).whenComplete((message, error) ->
                Platform.runLater(() -> {
                    if (closed) {
                        return;
                    }
                    if (error == null) {
                        timeline.accept(message);
                        return;
                    }
                    String notice = CoreMessages.forSend(error);
                    if (notice.equals(CoreMessages.MESSAGE_NOT_SENT)) {
                        timeline.markFailed(entry.clientMessageId());
                    } else {
                        // Core lo rechazó: reintentarlo daría el mismo error.
                        timeline.removePending(entry.clientMessageId());
                        Dialogs.showError(messagesList, notice);
                    }
                }));
    }

    /** Al (re)unirse: recupera lo que llegó mientras no había conexión y reenvía lo pendiente. */
    private void onJoined() {
        setStatus(null);
        sendButton.setDisable(false);
        if (joinedBefore) {
            loadHistory();
        }
        joinedBefore = true;
        for (ChatEntry entry : timeline.pending()) {
            timeline.markRetrying(entry.clientMessageId());
            send(entry);
        }
    }

    @FXML
    private void onAttachFile() {
        // TODO CU-23: abrir FileChooser, validar tipo y tamaño y compartir el archivo en el canal
    }

    @FXML
    private void onCancelReply() {
        // TODO CU-17: ocultar replyBox (visible + managed) y descartar el mensaje referenciado
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

    /** Avisos de la red: pasan al hilo de JavaFX. */
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
                if (!closed && timeline.accept(message)) {
                    scrollToEnd();
                }
            });
        }

        @Override
        public void onDisconnected() {
            Platform.runLater(() -> {
                if (!closed) {
                    sendButton.setDisable(true);
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

    /** Autor y hora arriba, texto abajo; los pendientes se distinguen por texto, no solo por color. */
    private static final class MessageCell extends ListCell<ChatEntry> {
        private final Label author = new Label();
        private final Label time = new Label();
        private final Label text = new Label();
        private final VBox box = new VBox(2, new HBox(8, author, time), text);

        MessageCell() {
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
                return;
            }
            text.getStyleClass().removeAll("message-text", "message-pending", "message-deleted");
            if (entry.isPending()) {
                author.setText("Tú");
                time.setText(entry.failed() ? "No enviado · se reintentará" : "Enviando…");
                text.getStyleClass().add("message-pending");
            } else {
                Message message = entry.message();
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
