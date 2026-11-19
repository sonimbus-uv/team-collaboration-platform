package mx.uv.sonimbus.desktop.ui.profile;

import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.ScrollPane;
import javafx.scene.control.TextField;
import javafx.scene.control.ToggleButton;
import javafx.scene.control.ToggleGroup;
import javafx.scene.image.ImageView;
import javafx.scene.layout.HBox;
import javafx.scene.layout.StackPane;
import javafx.scene.layout.TilePane;

import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.password.ChangePasswordRequest;
import mx.uv.sonimbus.desktop.domain.password.PasswordError;
import mx.uv.sonimbus.desktop.domain.password.PasswordException;
import mx.uv.sonimbus.desktop.domain.password.PasswordService;
import mx.uv.sonimbus.desktop.domain.session.AuthenticationException;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.ui.auth.common.AuthMessages;
import mx.uv.sonimbus.desktop.ui.auth.common.PasswordMessages;
import mx.uv.sonimbus.desktop.ui.auth.login.LoginNotice;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.common.PasswordVisibility;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

import java.util.Objects;

public class ProfileController {

    private final SessionService sessions;
    private final PasswordService passwords;
    private boolean changingPassword;
    private final BackgroundTasks backgroundTasks;
    private final ViewManager viewManager;
    private boolean loggingOut;
    private boolean active = true;

    @FXML private ScrollPane profileScroll;
    @FXML private javafx.scene.Node googleLink;
    @FXML private GoogleLinkController googleLinkController;

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
    @FXML private TextField currentPasswordVisibleField;
    @FXML private ToggleButton showCurrentPasswordToggle;
    @FXML private PasswordField newPasswordField;
    @FXML private TextField newPasswordVisibleField;
    @FXML private ToggleButton showNewPasswordToggle;
    @FXML private PasswordField confirmPasswordField;
    @FXML private TextField confirmPasswordVisibleField;
    @FXML private ToggleButton showConfirmPasswordToggle;
    @FXML private HBox confirmPasswordErrorBox;
    @FXML private Label confirmPasswordErrorLabel;
    @FXML private Button updatePasswordButton;

    // Sesión
    @FXML private Button logoutButton;
    @FXML private StackPane logoutConfirmOverlay;
    @FXML private Button cancelLogoutButton;
    @FXML private Button confirmLogoutButton;
    @FXML private HBox logoutErrorBox;
    @FXML private Label logoutErrorIconLabel;
    @FXML private Label logoutErrorLabel;

    public ProfileController(
            SessionService sessions,
            PasswordService passwords,
            BackgroundTasks backgroundTasks,
            ViewManager viewManager) {
        this.sessions = Objects.requireNonNull(sessions);
        this.passwords = Objects.requireNonNull(passwords);
        this.backgroundTasks = Objects.requireNonNull(backgroundTasks);
        this.viewManager = Objects.requireNonNull(viewManager);
    }

    @FXML
    private void initialize() {
        PasswordVisibility.bind(currentPasswordField, currentPasswordVisibleField);
        PasswordVisibility.bind(newPasswordField, newPasswordVisibleField);
        PasswordVisibility.bind(confirmPasswordField, confirmPasswordVisibleField);
        emailLabel.setText("Cuenta autenticada con Google");
        displayNameLabel.setText("Cuenta Google");
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
                                clearPasswordFields();
                            }
                        });
        for (TextField field :
                new TextField[] {
                    currentPasswordField, currentPasswordVisibleField,
                    newPasswordField, newPasswordVisibleField,
                    confirmPasswordField, confirmPasswordVisibleField
                }) {
            field.textProperty()
                    .addListener(
                            (observable, previous, current) -> {
                                field.getStyleClass().remove("field-invalid");
                                AlertBox.hide(passwordErrorBox);
                                AlertBox.hide(confirmPasswordErrorBox);
                                AlertBox.hide(passwordSavedBox);
                            });
            field.focusedProperty()
                    .addListener(
                            (observable, previous, focused) -> {
                                if (focused) {
                                    saveProfileButton.setDefaultButton(false);
                                    updatePasswordButton.setDefaultButton(true);
                                }
                            });
        }
        googleLinkController
                .busyProperty()
                .addListener(
                        (observable, previous, busy) -> {
                            logoutButton.setDisable(
                                    busy
                                            || changingPassword
                                            || loggingOut
                                            || !sessions.isAuthenticated());
                            updatePasswordButton.setDisable(
                                    busy
                                            || changingPassword
                                            || loggingOut
                                            || !sessions.isAuthenticated());
                        });
        logoutButton.setDisable(googleLinkController.isBusy() || !sessions.isAuthenticated());
        updatePasswordButton.setDisable(
                googleLinkController.isBusy() || !sessions.isAuthenticated());
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
    private void onToggleCurrentPasswordVisibility() {
        PasswordVisibility.apply(
                showCurrentPasswordToggle, currentPasswordField, currentPasswordVisibleField);
        showCurrentPasswordToggle.setAccessibleText(
                showCurrentPasswordToggle.isSelected()
                        ? "Ocultar contraseña actual"
                        : "Mostrar contraseña actual");
    }

    @FXML
    private void onToggleNewPasswordVisibility() {
        PasswordVisibility.apply(showNewPasswordToggle, newPasswordField, newPasswordVisibleField);
        showNewPasswordToggle.setAccessibleText(
                showNewPasswordToggle.isSelected()
                        ? "Ocultar nueva contraseña"
                        : "Mostrar nueva contraseña");
    }

    @FXML
    private void onToggleConfirmPasswordVisibility() {
        PasswordVisibility.apply(
                showConfirmPasswordToggle, confirmPasswordField, confirmPasswordVisibleField);
        showConfirmPasswordToggle.setAccessibleText(
                showConfirmPasswordToggle.isSelected()
                        ? "Ocultar confirmación de contraseña"
                        : "Mostrar confirmación de contraseña");
    }

    @FXML
    private void onUpdatePassword() {
        if (changingPassword || loggingOut || googleLinkController.isBusy() || !active) {
            return;
        }
        ChangePasswordRequest request =
                new ChangePasswordRequest(
                        currentPasswordField.getText(),
                        newPasswordField.getText(),
                        confirmPasswordField.getText());
        setChangingPassword(true);
        backgroundTasks.run(
                () -> {
                    passwords.changePassword(request);
                    return null;
                },
                ignored -> viewManager.show(View.LOGIN, LoginNotice.PASSWORD_UPDATED),
                this::onPasswordChangeFailed);
    }

    private void onPasswordChangeFailed(Throwable failure) {
        if (failure instanceof PasswordException passwordFailure && !sessions.isAuthenticated()) {
            LoginNotice notice =
                    passwordFailure.kind() == PasswordError.UNCERTAIN
                            ? LoginNotice.PASSWORD_UNCERTAIN
                            : LoginNotice.SESSION_EXPIRED;
            viewManager.show(View.LOGIN, notice);
            return;
        }
        if (!active) {
            return;
        }
        setChangingPassword(false);
        String message = AuthMessages.UNEXPECTED;
        if (failure instanceof PasswordException passwordFailure) {
            message = PasswordMessages.failure(passwordFailure);
            String confirmation = AuthMessages.forConfirmation(passwordFailure.problems());
            if (confirmation != null) {
                confirmPasswordErrorLabel.setText(confirmation);
                AlertBox.setShown(confirmPasswordErrorBox, true);
            }
            if (passwordFailure.kind() == PasswordError.INVALID_CURRENT) {
                markInvalidCurrentPassword();
            }
        } else {
            AuthLog.unexpectedFailure("change-password", failure);
        }
        passwordErrorLabel.setText(message);
        AlertBox.setShown(passwordErrorBox, true);
    }

    private void markInvalidCurrentPassword() {
        for (TextField field :
                new TextField[] {currentPasswordField, currentPasswordVisibleField}) {
            if (!field.getStyleClass().contains("field-invalid")) {
                field.getStyleClass().add("field-invalid");
            }
        }
        (showCurrentPasswordToggle.isSelected()
                        ? currentPasswordVisibleField
                        : currentPasswordField)
                .requestFocus();
    }

    private void setChangingPassword(boolean busy) {
        changingPassword = busy;
        googleLink.setDisable(busy);
        AlertBox.hide(passwordErrorBox);
        AlertBox.hide(confirmPasswordErrorBox);
        AlertBox.hide(passwordSavedBox);
        BusyButton.setBusy(updatePasswordButton, busy, "Actualizando…");
        currentPasswordField.setEditable(!busy);
        newPasswordField.setEditable(!busy);
        confirmPasswordField.setEditable(!busy);
        currentPasswordVisibleField.setEditable(!busy);
        newPasswordVisibleField.setEditable(!busy);
        confirmPasswordVisibleField.setEditable(!busy);
        showCurrentPasswordToggle.setDisable(busy);
        showNewPasswordToggle.setDisable(busy);
        showConfirmPasswordToggle.setDisable(busy);
        logoutButton.setDisable(busy);
        saveProfileButton.setDisable(busy);
        discardProfileButton.setDisable(busy);
    }

    private void clearPasswordFields() {
        currentPasswordField.clear();
        newPasswordField.clear();
        confirmPasswordField.clear();
    }

    @FXML
    private void onLogout() {
        if (changingPassword || loggingOut || googleLinkController.isBusy()) {
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
