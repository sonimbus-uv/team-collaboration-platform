package mx.uv.sonimbus.desktop.ui.navigation;

/**
 * Controlador que recibe datos de la vista que lo abre, por ejemplo el correo registrado hacia
 * {@link View#VERIFY_ACCOUNT}. {@link ViewManager} llama a {@link #setParameters} después de
 * {@code initialize()} y antes de mostrar la vista.
 *
 * <p>Si la vista se abre sin parámetros (por ejemplo, desde la navegación de demostración), no
 * se llama a {@link #setParameters}: el controlador debe mostrarse en un estado válido sin ellos.
 *
 * @param <P> tipo de los parámetros; conviene un {@code record} inmutable
 */
public interface Parameterized<P> {

    /** Tipo que acepta {@link #setParameters}; se usa para dar un error claro si no coincide. */
    Class<P> parametersType();

    void setParameters(P parameters);
}
