package mx.uv.sonimbus.desktop.ui.profile;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.ScrollPane;
import javafx.scene.control.TextField;
import javafx.scene.control.ToggleGroup;
import javafx.scene.image.ImageView;
import javafx.scene.layout.HBox;
import javafx.scene.layout.StackPane;
import javafx.scene.layout.TilePane;

public class ProfileController {

    @FXML private ScrollPane profileScroll;

    // Datos del perfil
    @FXML private HBox profileSavedBox;
    @FXML private Label profileSavedLabel;
    @FXML private HBox profileErrorBox;
    @FXML private Label profileErrorLabel;
    @FXML private Label avatarInitialsLabel;
    @FXML private ImageView currentAvatarView;
    @FXML private Label displayNameLabel;
    @FXML private Label emailLabel;
    @FXML private TextField nameField;
    @FXML private ToggleGroup avatarGroup;
    @FXML private TilePane avatarsPane;
    @FXML private Button saveProfileButton;
    @FXML private Button discardProfileButton;

    // Cambiar contraseña
    @FXML private HBox passwordSavedBox;
    @FXML private Label passwordSavedLabel;
    @FXML private HBox passwordErrorBox;
    @FXML private Label passwordErrorLabel;
    @FXML private PasswordField currentPasswordField;
    @FXML private PasswordField newPasswordField;
    @FXML private PasswordField confirmPasswordField;
    @FXML private HBox confirmPasswordErrorBox;
    @FXML private Label confirmPasswordErrorLabel;
    @FXML private Button updatePasswordButton;

    // Sesión
    @FXML private Button logoutButton;
    @FXML private StackPane logoutConfirmOverlay;
    @FXML private Button cancelLogoutButton;
    @FXML private Button confirmLogoutButton;

    @FXML
    private void initialize() {
        // TODO CU-07: cargar displayNameLabel, emailLabel, nameField y la foto actual
        //             (currentAvatarView con clip circular, o iniciales en avatarInitialsLabel)
        // TODO CU-07: llenar avatarsPane con un ToggleButton ("avatar-option") por cada imagen
        //             del paquete predefinido (resources/.../images/avatars), todos en
        //             avatarGroup, con accessibleText "Avatar N" y Tooltip; la opción elegida
        //             se distingue por el borde y una palomita, no solo por color. No se
        //             permite subir archivos
        // TODO CU-07: ocultar los avisos y quitar "field-invalid" al editar los campos
        // TODO: saveProfileButton es el defaultButton; pasarlo a updatePasswordButton mientras
        //       el foco esté en los campos de contraseña y a confirmLogoutButton mientras
        //       logoutConfirmOverlay esté visible
    }

    @FXML
    private void onSaveProfile() {
        // TODO CU-07: validar nameField (profileErrorBox), guardar nombre visible y foto
        //             elegida, mostrar profileSavedBox y actualizar el avatar del shell
    }

    @FXML
    private void onDiscardProfile() {
        // TODO CU-07: restaurar el nombre y la foto guardados
    }

    @FXML
    private void onUpdatePassword() {
        // TODO CU-07: validar contraseña actual, reglas de la nueva y coincidencia
        //             ("field-invalid" + confirmPasswordErrorBox / passwordErrorBox); si todo
        //             es válido, cambiarla, limpiar los campos y mostrar passwordSavedBox
    }

    @FXML
    private void onLogout() {
        // TODO CU-06: mostrar logoutConfirmOverlay (visible + managed), deshabilitar
        //             profileScroll y dar el foco a cancelLogoutButton
    }

    @FXML
    private void onCancelLogout() {
        // TODO CU-06: ocultar logoutConfirmOverlay y devolver el foco a logoutButton
    }

    @FXML
    private void onConfirmLogout() {
        // TODO CU-06: cerrar sesión (invalidar el token de renovación) y
        //             ViewManager.show(View.LOGIN)
    }
}
