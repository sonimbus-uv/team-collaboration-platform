package mx.uv.sonimbus.desktop.ui.navigation;

import javafx.scene.Parent;

/** Controlador de una vista que muestra otras vistas en su área central (el shell). */
public interface ContentHost {

    void setContent(Parent content);

    /** Quita la vista del área central y vuelve al estado inicial. */
    void clearContent();
}
