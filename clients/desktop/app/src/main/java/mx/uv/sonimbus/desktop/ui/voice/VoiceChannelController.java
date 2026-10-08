package mx.uv.sonimbus.desktop.ui.voice;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.ToggleButton;
import javafx.scene.layout.HBox;
import javafx.scene.layout.TilePane;
import javafx.scene.layout.VBox;
import javafx.scene.shape.SVGPath;

public class VoiceChannelController {

    // Avisos
    @FXML private VBox noticesBox;
    @FXML private HBox unavailableBox;
    @FXML private Label unavailableLabel;
    @FXML private Button retryButton;
    @FXML private HBox listenOnlyBox;
    @FXML private Label listenOnlyLabel;

    // Participantes
    @FXML private Label participantsCountLabel;
    @FXML private TilePane participantsPane;

    // Controles de la llamada
    @FXML private ToggleButton muteToggle;
    @FXML private SVGPath muteIcon;
    @FXML private Button leaveButton;

    @FXML
    private void initialize() {
        // TODO CU-22: llenar participantsPane con una tarjeta por participante (VBox
        //             "participant-card"; "participant-card" + "self" para el usuario actual):
        //             avatar ("avatar-large" con iniciales e ImageView), nombre
        //             ("participant-name") y estado del micrófono con icono + texto
        //             ("status-chip" con "Silenciado" o "Micrófono activo"); modelo pendiente
        // TODO CU-22: actualizar participantsCountLabel ("PARTICIPANTES (N)")
        // TODO CU-22: mostrar noticesBox + unavailableBox si no se puede conectar con la voz
        // TODO CU-22: mostrar noticesBox + listenOnlyBox y deshabilitar muteToggle cuando el
        //             usuario no tenga permiso para transmitir audio
    }

    @FXML
    private void onToggleMute() {
        // TODO CU-22: silenciar o activar el micrófono según muteToggle.isSelected();
        //             cambiar el texto a "Activar micrófono"/"Silenciar" y muteIcon al icono
        //             de micrófono tachado (mismo path + " M3 3L21 21")
    }

    @FXML
    private void onLeave() {
        // TODO CU-22: salir del canal de voz y volver a la vista anterior del shell
    }

    @FXML
    private void onRetry() {
        // TODO CU-22: reintentar la conexión con el servicio de voz y ocultar unavailableBox
    }
}
