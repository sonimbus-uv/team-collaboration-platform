package mx.uv.sonimbus.desktop.ui.groups;

import java.util.function.Consumer;
import mx.uv.sonimbus.desktop.data.core.Group;

public record GroupDialogParams(Consumer<Group> onGroup) {
}
