package mx.uv.sonimbus.desktop.ui.groups;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.TextArea;
import javafx.scene.control.TextField;
import javafx.scene.image.ImageView;
import javafx.scene.layout.HBox;

public class CreateGroupController {

    @FXML private HBox errorBox;
    @FXML private Label errorLabel;
    @FXML private TextField nameField;
    @FXML private TextArea descriptionArea;
    @FXML private Label descriptionCountLabel;
    @FXML private Label imageInitialsLabel;
    @FXML private ImageView groupImageView;
    @FXML private Button chooseImageButton;
    @FXML private Button removeImageButton;
    @FXML private Button cancelButton;
    @FXML private Button createButton;

    @FXML
    private void initialize() {
        // TODO CU-08: limitar descriptionArea a 200 caracteres (TextFormatter) y actualizar
        //             descriptionCountLabel ("N / 200")
        // TODO CU-08: mostrar en imageInitialsLabel las iniciales de nameField mientras no haya imagen
        // TODO CU-08: ocultar errorBox y quitar "field-invalid" de nameField al editar
    }

    @FXML
    private void onChooseImage() {
        // TODO CU-08: FileChooser de imágenes; mostrar la elegida en groupImageView (clip
        //             redondeado) y hacer visible removeImageButton
    }

    @FXML
    private void onRemoveImage() {
        // TODO CU-08: quitar la imagen elegida y ocultar removeImageButton
    }

    @FXML
    private void onCreateGroup() {
        // TODO CU-08: validar que nameField no esté vacío ("field-invalid" + errorBox); crear el
        //             grupo (el usuario queda como administrador), cerrar el diálogo y
        //             seleccionar el grupo nuevo en el shell
    }

    @FXML
    private void onCancel() {
        // TODO CU-08: cerrar el diálogo (Stage) sin crear el grupo
    }
}
