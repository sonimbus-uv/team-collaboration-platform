package mx.uv.sonimbus.desktop.ui.navigation;

/** Vistas de la aplicación; la ruta es relativa a resources/mx/uv/sonimbus/desktop/ui/. */
public enum View {
    LOGIN("auth/login.fxml"),
    REGISTER("auth/register.fxml"),
    VERIFY_ACCOUNT("auth/verify-account.fxml"),
    RECOVER_PASSWORD("auth/recover-password.fxml"),
    MAIN("shell/main.fxml"),
    TEXT_CHANNEL("chat/text-channel.fxml"),
    VOICE_CHANNEL("voice/voice-channel.fxml"),
    SHARED_FILES("files/shared-files.fxml"),
    CREATE_GROUP("groups/create-group.fxml"),
    JOIN_GROUP("groups/join-group.fxml"),
    GROUP_SETTINGS("groups/group-settings.fxml"),
    PROJECTS("projects/projects.fxml"),
    TASK_DETAIL("projects/task-detail.fxml"),
    REPORTS("reports/reports.fxml"),
    NOTIFICATIONS("notifications/notifications.fxml"),
    PROFILE("profile/profile.fxml");

    private final String fxml;

    View(String fxml) {
        this.fxml = fxml;
    }

    public String fxml() {
        return fxml;
    }
}
