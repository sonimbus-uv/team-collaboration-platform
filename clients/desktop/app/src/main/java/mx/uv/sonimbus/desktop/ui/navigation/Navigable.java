package mx.uv.sonimbus.desktop.ui.navigation;

/** Controlador que navega a otras vistas; {@link ViewManager} se lo entrega al cargarlo. */
public interface Navigable {

    /** Se invoca después de {@code initialize()}. */
    void setViewManager(ViewManager viewManager);
}
