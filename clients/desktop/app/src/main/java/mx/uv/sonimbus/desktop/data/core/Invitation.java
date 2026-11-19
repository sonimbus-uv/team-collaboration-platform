package mx.uv.sonimbus.desktop.data.core;

/** Invitación a un grupo (CU-10); {@code maxUses} nulo significa usos ilimitados. */
public record Invitation(
        String id,
        String groupId,
        String code,
        String expiresAt,
        Integer maxUses,
        int uses,
        String createdAt) {
}
