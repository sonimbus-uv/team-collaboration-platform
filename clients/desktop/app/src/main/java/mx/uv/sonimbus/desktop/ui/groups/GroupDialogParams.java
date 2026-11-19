package mx.uv.sonimbus.desktop.ui.groups;

import java.util.function.Consumer;
import mx.uv.sonimbus.desktop.data.core.Group;

/** Para los diálogos de crear y unirse: {@code onGroup} recibe el grupo para seleccionarlo. */
public record GroupDialogParams(Consumer<Group> onGroup) {
}
