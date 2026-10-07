package mx.uv.sonimbus.desktop.ui.navigation;

/** Vistas de la aplicación; la ruta es relativa a resources/mx/uv/sonimbus/desktop/ui/. */
public enum View {
    LOGIN("auth/login.fxml");

    private final String fxml;

    View(String fxml) {
        this.fxml = fxml;
    }

    public String fxml() {
        return fxml;
    }
}
