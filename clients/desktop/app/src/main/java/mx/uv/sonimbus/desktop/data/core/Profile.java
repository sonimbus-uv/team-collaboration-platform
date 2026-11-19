package mx.uv.sonimbus.desktop.data.core;

/** Copia local del perfil en Core; el nombre visible lo guarda el propio usuario (D-29). */
public record Profile(String userId, String displayName, String avatarId, String updatedAt) {
}
