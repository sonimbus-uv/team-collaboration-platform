package mx.uv.sonimbus.desktop.data.core;

public record Group(
        String id,
        String name,
        String description,
        String imageId,
        String createdBy,
        String role,
        String archivedAt,
        String createdAt) {

    public boolean isAdmin() {
        return "admin".equals(role);
    }

    public boolean canInvite() {
        return "admin".equals(role) || "moderator".equals(role);
    }
}
