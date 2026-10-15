package mx.uv.sonimbus.desktop.data.api;

/** Cuerpo de error común de los servicios (sección 14 de STANDARD_DEVELOPMENT.md). */
record ProblemDetails(String type, String title, int status, String detail) {
}
