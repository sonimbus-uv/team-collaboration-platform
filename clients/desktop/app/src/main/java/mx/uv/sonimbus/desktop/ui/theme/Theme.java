package mx.uv.sonimbus.desktop.ui.theme;

public enum Theme {
    LIGHT("Claro", "theme-light"),
    DARK("Oscuro", "theme-dark");

    private final String label;
    private final String styleClass;

    Theme(String label, String styleClass) {
        this.label = label;
        this.styleClass = styleClass;
    }

    String styleClass() {
        return styleClass;
    }

    @Override
    public String toString() {
        return label;
    }
}
