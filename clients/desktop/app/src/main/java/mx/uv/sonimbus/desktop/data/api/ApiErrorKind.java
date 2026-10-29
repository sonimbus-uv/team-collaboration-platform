package mx.uv.sonimbus.desktop.data.api;

/**
 * Casos de error que la interfaz necesita distinguir. Se deciden por código HTTP y operación,
 * nunca por el texto de {@code detail} (los servicios lo envían en inglés y no es contrato).
 */
public enum ApiErrorKind {
    /** 400 (o 422 en Core): el servicio rechazó los datos enviados. */
    VALIDATION,
    /** 409 en el registro: ya existe una cuenta con ese correo. */
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
    /** 500: el servicio falló al atender la petición. */
    SERVER,
    /** 502, 503 o 504: el servicio o una de sus dependencias no está disponible. */
    UNAVAILABLE,
    /** No hubo respuesta: sin conexión o servicio apagado. */
    NETWORK,
    /** Se agotó el tiempo de conexión o de respuesta (RNF-REN-01). */
    TIMEOUT,
    /** Respuesta que el cliente no sabe interpretar. */
    UNEXPECTED
}
