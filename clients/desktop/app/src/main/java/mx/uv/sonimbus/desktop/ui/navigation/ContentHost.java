package mx.uv.sonimbus.desktop.ui.navigation;

import javafx.scene.Parent;

/** Controlador de una vista que muestra otras vistas en su área central (el shell). */
public interface ContentHost {

    /**
     * Pone {@code content} en el área central. Recibe también la vista para actualizar lo que
     * dependa de ella (título de la sección, elemento seleccionado).
     */
    void setContent(View view, Parent content);

    /** Quita la vista del área central y vuelve al estado inicial. */
    void clearContent();
}
