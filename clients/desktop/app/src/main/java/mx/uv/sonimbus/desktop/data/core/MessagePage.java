package mx.uv.sonimbus.desktop.data.core;

import java.util.List;

public record MessagePage(List<Message> data, String nextCursor) {
}
