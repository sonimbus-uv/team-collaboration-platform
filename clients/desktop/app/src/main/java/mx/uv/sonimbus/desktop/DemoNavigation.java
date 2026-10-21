package mx.uv.sonimbus.desktop;

import java.util.EnumSet;
import java.util.List;
import java.util.Locale;
import java.util.Optional;
import java.util.Set;
import java.util.function.IntSupplier;
import mx.uv.sonimbus.desktop.ui.navigation.View;

/**
 * TEMPORAL: navegación de demostración para revisar y exponer las vistas sin tener todavía el
 * inicio de sesión ni los demás servicios. El primer argumento de la aplicación elige la vista:
 *
 * <ul>
 *   <li>{@code siguiente}: recorre las vistas en orden, una por ejecución;</li>
 *   <li>un número del 1 al {@code View.values().length};</li>
 *   <li>el nombre de la vista, sin importar mayúsculas y aceptando guiones ({@code verify-account}).</li>
 * </ul>
 *
 * Uso: {@code mvn -pl app javafx:run -Djavafx.args=siguiente} (ver el README).
 */
final class DemoNavigation {

    /** Cómo se presenta la vista elegida. */
    enum Placement {
        /** Ocupa toda la ventana (auth y shell). */
        FULL_WINDOW,
        /** Dentro del área central del shell. */
        CONTENT,
        /** Diálogo modal sobre el shell. */
        DIALOG
    }

    static final Set<View> FULL_WINDOW = EnumSet.of(
            View.LOGIN, View.REGISTER, View.VERIFY_ACCOUNT, View.RECOVER_PASSWORD, View.MAIN);

    static final Set<View> DIALOGS = EnumSet.of(View.CREATE_GROUP, View.JOIN_GROUP, View.TASK_DETAIL);

    private DemoNavigation() {
    }

    /**
     * Vista pedida en los argumentos, o vacío si no se pidió ninguna o no se reconoce.
     *
     * @param nextIndex da el índice de {@code siguiente} y avanza el contador (se guarda en
     *                  {@code Preferences} para que cada ejecución muestre la vista siguiente)
     */
    static Optional<View> requestedView(List<String> args, IntSupplier nextIndex) {
        if (args.isEmpty() || args.get(0).isBlank()) {
            return Optional.empty();
        }
        View[] views = View.values();
        String arg = args.get(0).strip();
        if (arg.equalsIgnoreCase("siguiente")) {
            return Optional.of(views[Math.floorMod(nextIndex.getAsInt(), views.length)]);
        }
        try {
            int number = Integer.parseInt(arg);
            return number >= 1 && number <= views.length ? Optional.of(views[number - 1]) : Optional.empty();
        } catch (NumberFormatException e) {
            // No es número: se intenta por nombre.
        }
        try {
            return Optional.of(View.valueOf(arg.toUpperCase(Locale.ROOT).replace('-', '_')));
        } catch (IllegalArgumentException e) {
            return Optional.empty();
        }
    }

    static Placement placementOf(View view) {
        if (FULL_WINDOW.contains(view)) {
            return Placement.FULL_WINDOW;
        }
        return DIALOGS.contains(view) ? Placement.DIALOG : Placement.CONTENT;
    }

    /** «Sonimbus · 3 de 16 · VERIFY_ACCOUNT». */
    static String windowTitle(View view) {
        return "Sonimbus · " + (view.ordinal() + 1) + " de " + View.values().length + " · " + view;
    }

    /** Título de la ventana de cada diálogo (los del shell usan los mismos). */
    static String dialogTitle(View view) {
        return switch (view) {
            case CREATE_GROUP -> "Crear grupo";
            case JOIN_GROUP -> "Unirse a un grupo";
            case TASK_DETAIL -> "Detalle de la tarea";
            default -> "Sonimbus";
        };
    }

    /** Lista de opciones para cuando el argumento no se reconoce. */
    static String usage(String arg) {
        StringBuilder text = new StringBuilder("Vista desconocida: ").append(arg).append(". Opciones:");
        for (View view : View.values()) {
            text.append(System.lineSeparator()).append("  ").append(view.ordinal() + 1).append("  ").append(view);
        }
        return text.toString();
    }
}
