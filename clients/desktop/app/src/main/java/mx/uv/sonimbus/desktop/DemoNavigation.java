package mx.uv.sonimbus.desktop;

import java.util.EnumSet;
import java.util.List;
import java.util.Locale;
import java.util.Optional;
import java.util.Set;
import java.util.function.IntSupplier;
import mx.uv.sonimbus.desktop.ui.navigation.View;

final class DemoNavigation {

    enum Placement {
        FULL_WINDOW,
        CONTENT,
        DIALOG
    }

    static final Set<View> FULL_WINDOW = EnumSet.of(
            View.LOGIN, View.REGISTER, View.VERIFY_ACCOUNT, View.RECOVER_PASSWORD, View.MAIN);

    static final Set<View> DIALOGS = EnumSet.of(View.CREATE_GROUP, View.JOIN_GROUP, View.TASK_DETAIL);

    private DemoNavigation() {
    }

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

    static String windowTitle(View view) {
        return "Sonimbus · " + (view.ordinal() + 1) + " de " + View.values().length + " · " + view;
    }

    static String dialogTitle(View view) {
        return switch (view) {
            case CREATE_GROUP -> "Crear grupo";
            case JOIN_GROUP -> "Unirse a un grupo";
            case TASK_DETAIL -> "Detalle de la tarea";
            default -> "Sonimbus";
        };
    }

    static String usage(String arg) {
        StringBuilder text = new StringBuilder("Vista desconocida: ").append(arg).append(". Opciones:");
        for (View view : View.values()) {
            text.append(System.lineSeparator()).append("  ").append(view.ordinal() + 1).append("  ").append(view);
        }
        return text.toString();
    }
}
