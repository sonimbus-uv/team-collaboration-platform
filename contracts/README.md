# Contratos entre servicios y clientes

Esta carpeta contiene acuerdos observables para desarrollar clientes y servicios
sin depender de la implementación interna de otro módulo. La existencia de un
contrato no acredita despliegue, integración ni aprobación de parámetros que el
propio documento identifica como pendientes.

## Auth

| Documento | Fuente principal de información |
|---|---|
| [API pública](auth/public-api.md) | Registro, sesiones, contraseñas, diseño Google/contacto y contenido del correo de recuperación |
| [Tokens](auth/tokens.md) | Credenciales, claims, JWKS y obligaciones de validación |
| [API interna](auth/internal-api.md) | Consulta privada de sesiones para Core y Files |
| [Evento session.revoked](events/session.revoked.md) | Envelope, entrega e idempotencia del evento de revocación |

La [documentación del servicio](../services/auth/README.md) concentra arquitectura,
configuración, desarrollo, operación y pruebas. Las decisiones y alternativas se
conservan en [ADR](../docs/decisions/README.md); los resultados de ejecuciones y
planes históricos no forman parte del contrato vigente.

El [contrato Google y contacto v1.0](auth/public-api.md#google-y-correo-de-contacto)
está aprobado como diseño y pendiente de implementación. Su semántica se mantiene
en ese documento único; los OpenAPI siguientes todavía no incluyen sus rutas
objetivo ni la nueva elegibilidad de sesión.

## OpenAPI

Los archivos usan OpenAPI 3.1 en sintaxis JSON compatible con YAML:

| Archivo | Alcance |
|---|---|
| [auth.yaml](openapi/auth.yaml) | Entrada completa de la API pública; compone las áreas mediante `$ref` y define JWKS |
| [auth-registration.yaml](openapi/auth-registration.yaml) | Registro, verificación y reenvío |
| [auth-sessions.yaml](openapi/auth-sessions.yaml) | Login, refresh y logout |
| [auth-passwords.yaml](openapi/auth-passwords.yaml) | Cambio y recuperación de contraseña |
| [auth-internal.yaml](openapi/auth-internal.yaml) | Consulta privada por HTTPS, independiente del contrato público |

Cada operación tiene una única definición en su archivo funcional. `auth.yaml`
referencia sus Path Items; las definiciones de esquemas conservan referencias
relativas al archivo que las declara. JWKS y el esquema común de errores públicos
permanecen en el documento principal. La variante de errores de contraseña
conserva su esquema propio y sus requisitos existentes.

La carga del contrato público completo comienza en `auth.yaml`. Las herramientas
deben resolver referencias relativas entre archivos; la distribución conserva la
carpeta `openapi/` completa o utiliza una versión empaquetada por una herramienta
compatible con OpenAPI 3.1. No existe una copia empaquetada mantenida manualmente.
La división de archivos no cambia las URL del servicio ni habilita funciones.

## Organización

`auth/` agrupa la semántica de Auth; `openapi/`, sus estructuras y estados HTTP;
`events/`, los acuerdos de eventos y sus esquemas. Los contratos de otros módulos
se incorporan cuando exista su primera especificación, sin anticipar archivos o
endpoints disponibles.

## Mantenimiento

OpenAPI define rutas, cuerpos y estados HTTP. Los documentos de dominio explican
reglas y obligaciones que esas estructuras no expresan por sí solas. Las guías
del servicio enlazan a ambas fuentes sin mantener copias de las tablas HTTP.
Una discrepancia requiere corrección o revisión explícita.

- Los cambios de contrato se revisan en un PR propio antes de implementarse en
  el servicio o sus consumidores.
- Los cambios incompatibles incrementan la versión del contrato o evento e
  identifican los clientes y servicios afectados.
- El responsable del módulo mantiene sus acuerdos conforme al
  [reparto de módulos](../docs/PLAN.md#reparto-de-módulos).
- Los formatos comunes siguen el [estándar de desarrollo](../docs/STANDARD_DEVELOPMENT.md).
- Las propuestas pendientes permanecen identificadas y separadas de las
  garantías implementadas. Los informes de ejecución pertenecen al PR o historial.
