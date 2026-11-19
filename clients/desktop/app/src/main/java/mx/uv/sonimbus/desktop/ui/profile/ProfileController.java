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

import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.ui.auth.common.AuthMessages;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.navigation.View;

public class ProfileController {
    private final mx.uv.sonimbus.desktop.domain.session.SessionService sessions;
    private final mx.uv.sonimbus.desktop.ui.common.BackgroundTasks backgroundTasks;
    private final mx.uv.sonimbus.desktop.ui.navigation.ViewManager viewManager;
    private boolean loggingOut;
    private boolean active = true;
    @FXML private HBox logoutErrorBox;
    @FXML private Label logoutErrorIconLabel;
    @FXML private Label logoutErrorLabel;

    public ProfileController(
            mx.uv.sonimbus.desktop.domain.session.SessionService sessions,
            mx.uv.sonimbus.desktop.ui.common.BackgroundTasks backgroundTasks,
            mx.uv.sonimbus.desktop.ui.navigation.ViewManager viewManager) {
        this.sessions = sessions;
        this.backgroundTasks = backgroundTasks;
        this.viewManager = viewManager;
    }

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
        sessions.email()
                .ifPresent(
                        email -> {
                            emailLabel.setText(email);
                            displayNameLabel.setText(email);
                        });
        logoutButton.setDisable(!sessions.isAuthenticated());
        profileScroll
                .sceneProperty()
                .addListener(
                        (observable, previous, current) -> {
                            if (previous != null && current == null) {
                                active = false;
                            }
                        });
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
        if (loggingOut) {
            return;
        }
        AlertBox.hide(logoutErrorBox);
        AlertBox.setShown(logoutConfirmOverlay, true);
        profileScroll.setDisable(true);
        saveProfileButton.setDefaultButton(false);
        confirmLogoutButton.setDefaultButton(true);
        cancelLogoutButton.requestFocus();
    }

    @FXML
    private void onCancelLogout() {
        if (loggingOut) {
            return;
        }
        AlertBox.setShown(logoutConfirmOverlay, false);
        profileScroll.setDisable(false);
        confirmLogoutButton.setDefaultButton(false);
        saveProfileButton.setDefaultButton(true);
        logoutButton.requestFocus();
    }

    @FXML
    private void onConfirmLogout() {
        if (loggingOut || !active) {
            return;
        }
        setLoggingOut(true);
        AlertBox.hide(logoutErrorBox);
        backgroundTasks.run(
                () -> {
                    sessions.logout();
                    return null;
                },
                ignored -> {
                    if (!sessions.isAuthenticated()) {
                        viewManager.show(View.LOGIN);
                    }
                },
                this::onLogoutFailed);
    }

    private void onLogoutFailed(Throwable failure) {
        if (!active) {
            return;
        }
        setLoggingOut(false);
        String message = "No se confirmó el cierre de sesión. Intenta de nuevo.";
        if (failure instanceof AuthenticationException authenticationFailure) {
            message = AuthMessages.forLogout(authenticationFailure);
        } else {
            AuthLog.unexpectedFailure("logout", failure);
        }
        AlertBox.show(
                logoutErrorBox,
                logoutErrorIconLabel,
                logoutErrorLabel,
                AlertBox.Kind.ERROR,
                message);
    }

    private void setLoggingOut(boolean busy) {
        loggingOut = busy;
        BusyButton.setBusy(confirmLogoutButton, busy, "Cerrando sesión…");
        cancelLogoutButton.setDisable(busy);
    }
}
