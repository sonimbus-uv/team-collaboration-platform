package mx.uv.sonimbus.desktop.data.core;

/** Canal de un grupo; {@code type} es {@code text} o {@code voice}. */
public record Channel(
        String id,
        String groupId,
        String name,
        String description,
        String type,
        String minRole,
        String createdAt) {

    public boolean isText() {
        return "text".equals(type);
    }
}
