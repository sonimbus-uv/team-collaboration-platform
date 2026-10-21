package mx.uv.sonimbus.desktop.data.api;

/**
 * Casos de error que la interfaz necesita distinguir. Se deciden por código HTTP y operación,
 * nunca por el texto de {@code detail} (los servicios lo envían en inglés y no es contrato).
 */
public enum ApiErrorKind {
    /** 400: el servicio rechazó los datos enviados. */
    VALIDATION,
    /** 409 en el registro: ya existe una cuenta con ese correo. */
    EMAIL_TAKEN,
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
