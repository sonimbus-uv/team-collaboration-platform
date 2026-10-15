package mx.uv.sonimbus.desktop.data.api;

/** Casos de error que la interfaz necesita distinguir para mostrar su mensaje. */
public enum ApiError {
    EMAIL_ALREADY_EXISTS,
    INVALID_EMAIL,
    INVALID_PASSWORD,
    INVALID_TOKEN,
    INVALID_REQUEST,
    TOO_MANY_REQUESTS,
    SERVICE_UNAVAILABLE,
    /** No hubo respuesta: sin conexión, servicio apagado o tiempo de espera agotado. */
    NETWORK,
    UNEXPECTED
}
