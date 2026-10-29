# Auth para móvil y escritorio

**Estado al 2026-10-28: contrato candidato para desarrollo con mocks, pendiente
de revisión técnica.** Este paquete permite construir clientes HTTP, pantallas
y pruebas simuladas; no acredita disponibilidad de un entorno ni autoriza
habilitar login para uso real. No constituye una aprobación nueva de los
parámetros que los documentos todavía identifican como propuestas.

## Documentos y orden de lectura

| Documento | Uso |
|---|---|
| [OpenAPI público](openapi/auth.yaml) | Cuerpos, respuestas y errores de login, refresh, logout y JWKS. El archivo contiene JSON válido, compatible con YAML. |
| [Flujos de sesión](auth-session-flows.md) | Login, rotación, restauración, logout y resultados inciertos. |
| [Perfil de tokens](token-profile.md) | Tipos de credenciales, claims, vencimientos y almacenamiento previsto. |
| [Contrato de registro](registration.md) | Registro con intento+código, reenvío y compatibilidad; incluido en OpenAPI 0.2.0. |
| [Registro: operación y pruebas](../services/auth/REGISTRATION.md) | Implementación y pruebas aisladas con Mailpit. |
| [Sesiones implementadas](../services/auth/SESSIONS.md) | Comportamiento del backend y bloqueo de habilitación por #17. |
| [ADR-007](../docs/decisions/ADR-007-auth-sessions.md) | Contexto de las decisiones de sesiones y revocación. |

Conservar las rutas del repositorio al compartir estos documentos para mantener
sus enlaces. El código Rust, migraciones y configuración del servidor no son
necesarios para comenzar con mocks.

## Alcance y pendientes

- Login, refresh, logout, JWT/JWKS y registro/verificación tienen implementación
  de backend. Login sigue deshabilitado por defecto por #17; no habilitarlo en
  un entorno compartido como consecuencia de esta publicación documental.
- Forgot/reset password están pendientes de diseño e implementación. No inventar
  rutas, cuerpos o respuestas para integrarlos como si fueran definitivos.
- Cambio incompatible aprobado 2026-10-28: el registro devuelve identificador de
  intento, no cuenta; la confirmación envía ID+código, no token. GET antiguo no
  activa cuentas. La aplicación conserva el ID de su propia solicitud. Ver
  [registration.md](registration.md). La solución local de #17 sigue en revisión;
  no sanea automáticamente cuentas verificadas por el flujo anterior.
- Los prefijos públicos actuales difieren: sesiones bajo `/api/v1/auth` y
  registro/verificación bajo `/auth`. No normalizarlos unilateralmente; la
  convergencia está pendiente en #14.
- La URL del OpenAPI es ficticia. URL real, entorno aislado y credenciales de
  prueba deben coordinarse antes de la integración.
- Los flujos de recuperación, el tratamiento de un llavero inaccesible y el
  fallo al guardar un refresh sucesor requieren acuerdos adicionales.
- La API interna de sesiones y sus credenciales pertenecen exclusivamente a
  Core/Files. Móvil y escritorio no deben invocarla ni recibir esos secretos.

## Reglas para los clientes nativos

El diseño previsto conserva el JWT de acceso en memoria y el refresh original
en el almacén seguro del sistema operativo. Auth entrega ambos en JSON; no
implementa ese almacenamiento del cliente ni emite cookies de sesión.
Las solicitudes protegidas presentan el access mediante `Authorization: Bearer`.
El refresh se envía únicamente a los flujos previstos de Auth, no a Core/Files.

Coordinar una única renovación en vuelo por sesión entre tareas/procesos que
compartan credencial. Tras éxito, sustituir el refresh almacenado antes de
permitir otra renovación. No registrar tokens, contraseñas ni cuerpos sensibles.

Un 503 explícito no significa revocación y no borra automáticamente el refresh.
En cambio, un timeout o desconexión tras enviar una renovación puede dejar un
resultado incierto: no reintentar automáticamente el refresh anterior. Seguir
la política de restauración de [los flujos](auth-session-flows.md), que requiere
login ante ese resultado. Esta distinción debe estar cubierta por los mocks.

## Coordinación y revisión

Desarrollar inicialmente contra mocks del OpenAPI y ejemplos ficticios. Registrar
discrepancias entre documentos e implementación antes de decidir cuál cambiar;
no considerar una propuesta histórica como garantía operativa.

Antes de integrar con un entorno real, acordar los pendientes y revisar el
contrato público. Todo cambio incompatible debe comunicarse a ambos clientes
y versionarse conforme a las reglas de [contratos](README.md). Compartir solo
documentación y ejemplos sin secretos; excluir `.env`, claves privadas y
credenciales reales.
