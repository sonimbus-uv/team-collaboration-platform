package mx.uv.sonimbus.desktop.data.core;

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
