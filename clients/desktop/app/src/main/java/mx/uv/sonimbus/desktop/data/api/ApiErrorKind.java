package mx.uv.sonimbus.desktop.data.api;

public enum ApiErrorKind {
    /** 400 (o 422 en Core): el servicio rechazó los datos enviados. */
    VALIDATION,
    EMAIL_TAKEN,
    /** 401 en el inicio de sesión: correo o contraseña incorrectos (no se distingue cuál). */
    INVALID_CREDENTIALS,
    /** 401 fuera del inicio de sesión: la sesión venció o fue revocada; hay que iniciarla de nuevo. */
    UNAUTHORIZED,
    /** 403: el rol en el grupo no permite la acción. */
    FORBIDDEN,
    /** 403 al unirse: el usuario está bloqueado en el grupo (CU-11 EX-03). */
    BANNED,
    /** 404: no existe o el usuario no tiene acceso; Core no distingue. */
    NOT_FOUND,
    /** 409: nombre repetido, grupo archivado o perfil administrado por Auth. */
    CONFLICT,
    /** 422 al unirse: invitación inexistente, caducada, revocada o agotada (CU-11 EX-01). */
    INVALID_INVITATION,
    /** 429: hay que esperar {@link ApiException#retryAfter()} antes de reintentar. */
    RATE_LIMITED,
    SERVER,
    UNAVAILABLE,
    NETWORK,
    TIMEOUT,
    UNEXPECTED
}
