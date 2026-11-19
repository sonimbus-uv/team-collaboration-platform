package mx.uv.sonimbus.desktop.data.core;

import java.util.List;

/** Página del historial, del más reciente al más antiguo; {@code nextCursor} nulo al final. */
public record MessagePage(List<Message> data, String nextCursor) {
}
