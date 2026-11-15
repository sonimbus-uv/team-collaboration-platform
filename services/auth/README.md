# Servicio Auth

Auth administra cuentas, registro y verificación de correo, contraseñas, sesiones
y emisión de tokens. PostgreSQL es la autoridad de cuentas y sesiones; Redis
coordina la protección distribuida. El servicio utiliza Rust, Axum, Tokio y SQLx.

## Documentación

| Documento | Contenido y audiencia |
|---|---|
| [Arquitectura](docs/architecture.md) | Responsabilidades, flujos internos, persistencia y concurrencia para desarrollo |
| [Configuración](docs/configuration.md) | Variables, secretos y condiciones de arranque para desarrollo y operación |
| [Desarrollo local](docs/development.md) | Preparación de la VM, ejecución y acceso desde Postman |
| [Operación](docs/operations.md) | Claves, credenciales internas, protección de red y diagnóstico |
| [Pruebas](docs/testing.md) | Entorno aislado, comandos y cobertura |
| [Contratos de Auth](../../contracts/auth/public-api.md#alcance-y-pendientes) | Acuerdos públicos para clientes y servicios consumidores |
| [Decisiones](../../docs/decisions/README.md) | Justificación de decisiones arquitectónicas |
| [Validaciones históricas](docs/history.md) | Evidencia de entregas anteriores; no describe el estado vigente |

## Estado funcional

El backend incluye registro por intento y código, login, rotación de refresh,
logout, JWT/JWKS, cambio y recuperación de contraseña, y consulta privada de
sesiones por HTTPS. La intención de revocación se persiste en outbox; la
publicación RabbitMQ y la integración de consumidores siguen pendientes.

Login, contraseñas y API interna tienen banderas independientes. Login y
contraseñas permanecen deshabilitados por defecto; el script de desarrollo
aprovisiona una API interna limitada a loopback. La existencia de código no
acredita despliegue ni habilitación para producción. La revisión de cuentas
heredadas y de enumeración sigue siendo una condición operativa.

El [contrato Google y correo de contacto v1.0](../../contracts/auth/public-api.md#google-y-correo-de-contacto)
define el diseño aprobado. El esquema, adaptador OIDC y login Google están
implementados con activación explícita, junto con vinculación, conexiones y
reautenticación por contraseña. La verificación general de contacto y el worker
de avisos están implementados con banderas independientes, apagadas por defecto.
La integración de clientes y el aprovisionamiento Google real requieren validación.
La gestión general de perfil tampoco forma parte de los flujos implementados.
Los requisitos generales se encuentran en
[requisitos](../../docs/requirements/README.md#auth-service).

## Mantenimiento documental

Los contratos contienen entradas, respuestas, errores y garantías observables.
Las guías del servicio describen implementación y operación, enlazando al contrato
correspondiente sin repetir sus tablas HTTP. Los ADR conservan decisiones y
alternativas. Los resultados de una ejecución pertenecen al PR, al issue o al
archivo histórico, separados de las instrucciones vigentes.

Cada cambio de comportamiento actualiza su fuente documental principal y los
enlaces afectados. Los valores de referencia se mantienen en
[.env.example](.env.example), las estructuras HTTP en OpenAPI y las versiones de
dependencias en [Cargo.toml](Cargo.toml) y [Cargo.lock](Cargo.lock).
