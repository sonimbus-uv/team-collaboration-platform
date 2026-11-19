package mx.uv.sonimbus.desktop.ui.profile;

import javafx.beans.property.ReadOnlyBooleanProperty;
import javafx.beans.property.ReadOnlyBooleanWrapper;
import javafx.beans.property.ReadOnlyStringProperty;
import javafx.beans.property.ReadOnlyStringWrapper;
import javafx.fxml.FXML;
import javafx.scene.control.Button;
import javafx.scene.control.Label;
import javafx.scene.control.PasswordField;
import javafx.scene.control.TextField;
import javafx.scene.control.ToggleButton;
import javafx.scene.layout.HBox;
import javafx.scene.layout.VBox;

import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.google.GoogleConnection;
import mx.uv.sonimbus.desktop.domain.google.GoogleError;
import mx.uv.sonimbus.desktop.domain.google.GoogleException;
import mx.uv.sonimbus.desktop.domain.google.GoogleService;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.ui.auth.common.AuthMessages;
import mx.uv.sonimbus.desktop.ui.auth.common.GoogleMessages;
import mx.uv.sonimbus.desktop.ui.auth.login.LoginNotice;
import mx.uv.sonimbus.desktop.ui.common.AlertBox;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.common.BusyButton;
import mx.uv.sonimbus.desktop.ui.common.PasswordVisibility;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

import java.util.Objects;
import java.util.Optional;

public final class GoogleLinkController {
    private final GoogleService google;
    private final SessionService sessions;
    private final BackgroundTasks tasks;
    private final ViewManager views;
    private final ReadOnlyBooleanWrapper busy = new ReadOnlyBooleanWrapper();
    private final ReadOnlyStringWrapper linkedEmail = new ReadOnlyStringWrapper("");
    private boolean active = true;
    private boolean linking;
    private boolean linked;
    private boolean reauthenticationRequired;

    @FXML private VBox googleLinkBox;
    @FXML private Label googleConnectionLabel;
    @FXML private HBox googleLinkMessageBox;
    @FXML private Label googleLinkMessageIcon;
    @FXML private Label googleLinkMessageLabel;
    @FXML private VBox googleReauthenticationBox;
    @FXML private PasswordField googlePasswordField;
    @FXML private TextField googlePasswordVisibleField;
    @FXML private ToggleButton showGooglePasswordToggle;
    @FXML private Button linkGoogleButton;
    @FXML private Button refreshGoogleConnectionButton;
    @FXML private Button cancelGoogleLinkButton;

    public GoogleLinkController(
            GoogleService google,
            SessionService sessions,
            BackgroundTasks tasks,
            ViewManager views) {
        this.google = Objects.requireNonNull(google);
        this.sessions = Objects.requireNonNull(sessions);
        this.tasks = Objects.requireNonNull(tasks);
        this.views = Objects.requireNonNull(views);
    }

    @FXML
    private void initialize() {
        PasswordVisibility.bind(googlePasswordField, googlePasswordVisibleField);
        googleLinkBox
                .sceneProperty()
                .addListener(
                        (observable, previous, current) -> {
                            if (previous != null && current == null) {
                                active = false;
                                googlePasswordField.clear();
                                if (linking) {
                                    google.cancel();
                                }
                            }
                        });
        if (sessions.isAuthenticated()) {
            loadConnection();
        } else {
            googleConnectionLabel.setText("Inicia sesión para gestionar la vinculación.");
            setBusy(false);
        }
    }

    public ReadOnlyBooleanProperty busyProperty() {
        return busy.getReadOnlyProperty();
    }

    public boolean isBusy() {
        return busy.get();
    }

    ReadOnlyStringProperty linkedEmailProperty() {
        return linkedEmail.getReadOnlyProperty();
    }

    @FXML
    private void onRefreshGoogleConnection() {
        if (!busy.get() && active) {
            loadConnection();
        }
    }

    private void loadConnection() {
        setBusy(true);
        tasks.run(
                google::connection,
                this::showConnection,
                failure -> {
                    if (!active) {
                        return;
                    }
                    setBusy(false);
                    googleConnectionLabel.setText("No se pudo consultar la vinculación.");
                    showFailure(failure);
                });
    }

    private void showConnection(Optional<GoogleConnection> connection) {
        if (!active) {
            return;
        }
        linked = connection.isPresent();
        String email = connection.map(GoogleConnection::providerEmail)
                .filter(value -> !value.isBlank())
                .orElse("");
        linkedEmail.set(email);
        String label = "Google no está vinculado a esta cuenta.";
        if (linked) {
            label = email.isEmpty() ? "Google vinculado." : "Google vinculado: " + email;
        }
        googleConnectionLabel.setText(label);
        setBusy(false);
    }

    @FXML
    private void onLinkGoogle() {
        if (!active || busy.get() || linked) {
            return;
        }
        String password = reauthenticationRequired ? googlePasswordField.getText() : null;
        googlePasswordField.clear();
        linking = true;
        AlertBox.hide(googleLinkMessageBox);
        setBusy(true);
        tasks.run(
                () -> {
                    google.link(password);
                    return null;
                },
                ignored -> onLinked(),
                this::onLinkFailed);
    }

    private void onLinked() {
        if (!active) {
            return;
        }
        linking = false;
        linked = true;
        reauthenticationRequired = false;
        AlertBox.setShown(googleReauthenticationBox, false);
        googleConnectionLabel.setText("Google vinculado.");
        setBusy(false);
        AlertBox.show(
                googleLinkMessageBox,
                googleLinkMessageIcon,
                googleLinkMessageLabel,
                AlertBox.Kind.SUCCESS,
                "Google quedó vinculado. Ya puedes usarlo para iniciar sesión en esta cuenta.");
        views.bringToFront();
        loadConnection();
    }

    private void onLinkFailed(Throwable failure) {
        if (!active) {
            return;
        }
        linking = false;
        if (failure instanceof GoogleException googleFailure) {
            if (googleFailure.kind() == GoogleError.REAUTHENTICATION_REQUIRED) {
                reauthenticationRequired = true;
                AlertBox.setShown(googleReauthenticationBox, true);
            }
            if (googleFailure.kind() == GoogleError.ALREADY_LINKED) {
                linked = true;
            }
        }
        setBusy(false);
        showFailure(failure);
        if (reauthenticationRequired) {
            (showGooglePasswordToggle.isSelected()
                            ? googlePasswordVisibleField
                            : googlePasswordField)
                    .requestFocus();
        }
        views.bringToFront();
    }

    private void showFailure(Throwable failure) {
        if (failure instanceof GoogleException googleFailure
                && googleFailure.kind() == GoogleError.NO_SESSION) {
            views.show(View.LOGIN, LoginNotice.SESSION_EXPIRED);
            return;
        }
        String message = AuthMessages.UNEXPECTED;
        if (failure instanceof GoogleException googleFailure) {
            message = GoogleMessages.failure(googleFailure);
        } else {
            AuthLog.unexpectedFailure("google-link", failure);
        }
        AlertBox.show(
                googleLinkMessageBox,
                googleLinkMessageIcon,
                googleLinkMessageLabel,
                AlertBox.Kind.ERROR,
                message);
    }

    private void setBusy(boolean value) {
        busy.set(value);
        BusyButton.setBusy(linkGoogleButton, value && linking, "Esperando a Google…");
        linkGoogleButton.setDisable(value || linked || !sessions.isAuthenticated());
        refreshGoogleConnectionButton.setDisable(value || !sessions.isAuthenticated());
        googlePasswordField.setEditable(!value);
        googlePasswordVisibleField.setEditable(!value);
        showGooglePasswordToggle.setDisable(value);
        AlertBox.setShown(cancelGoogleLinkButton, value && linking);
        if (!value) {
            linkGoogleButton.setText(
                    linked
                            ? "Google vinculado"
                            : reauthenticationRequired
                                    ? "Confirmar y vincular Google"
                                    : "Vincular Google");
        }
    }

    @FXML
    private void onCancelGoogleLink() {
        google.cancel();
    }

    @FXML
    private void onToggleGooglePasswordVisibility() {
        PasswordVisibility.apply(
                showGooglePasswordToggle, googlePasswordField, googlePasswordVisibleField);
    }
}
