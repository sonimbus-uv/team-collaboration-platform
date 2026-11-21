package mx.uv.sonimbus.desktop.data.core;

public record Invitation(
        String id,
        String groupId,
        String code,
        String expiresAt,
        Integer maxUses,
        int uses,
        String createdAt) {
}
