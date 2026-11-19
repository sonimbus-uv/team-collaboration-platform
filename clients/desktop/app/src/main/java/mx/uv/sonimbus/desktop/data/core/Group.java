package mx.uv.sonimbus.desktop.data.core;

/** Grupo con el rol del usuario ({@code member}, {@code moderator} o {@code admin}). */
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

    /** Moderador o administrador: puede crear invitaciones (CU-10). */
    public boolean canInvite() {
        return "admin".equals(role) || "moderator".equals(role);
    }
}
