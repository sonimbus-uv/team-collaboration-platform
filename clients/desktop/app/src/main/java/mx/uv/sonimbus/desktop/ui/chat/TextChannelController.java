package mx.uv.sonimbus.desktop.ui.chat;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ListView;
import javafx.scene.control.TextField;
import javafx.scene.layout.HBox;

public class TextChannelController {

    // Historial
    @FXML private ListView<Object> messagesList;         // TODO: modelo de mensaje
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

    @FXML
    private void initialize() {
        // TODO CU-20: cell factory de messagesList: separador de día ("day-separator"), avatar,
        //             autor ("message-author"), hora ("message-time"), texto ("message-text"),
        //             marca "(editado)", "Mensaje eliminado" ("message-deleted"), referencia al
        //             mensaje respondido ("reply-reference"), reacciones ("reaction-chip" y
        //             "reaction-chip" + "selected"), archivo adjunto ("attachment-card" con
        //             botón Descargar, CU-23) y estado "Enviando…" ("message-pending")
        // TODO CU-20: cargar el historial por páginas al desplazarse hacia arriba y mostrar
        //             emptyMessagesLabel cuando el canal no tenga mensajes
        // TODO CU-17, CU-18, CU-19: menú contextual por mensaje (ContextMenu en código) con
        //             Responder, Reaccionar, Editar y Eliminar; Editar y Eliminar solo en
        //             mensajes propios
        // TODO CU-16: messageLabel con el nombre del canal ("Mensaje para #canal")
    }

    @FXML
    private void onSend() {
        // TODO CU-16: enviar el texto de messageField (con client_message_id); si replyBox está
        //             visible, enviarlo como respuesta (CU-17) y ocultar la barra; limpiar el campo
    }

    @FXML
    private void onAttachFile() {
        // TODO CU-23: abrir FileChooser, validar tipo y tamaño y compartir el archivo en el canal
    }

    @FXML
    private void onCancelReply() {
        // TODO CU-17: ocultar replyBox (visible + managed) y descartar el mensaje referenciado
    }
}
