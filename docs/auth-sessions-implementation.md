# Plan de implementación y pruebas de sesiones

Primera entrega documental, 2026-10-22. Refs #35.
[ADR-007](decisions/ADR-007-auth-sessions.md), [contratos](../contracts/README.md).
No habilita endpoints ni modifica migraciones, servicios, clientes o gateway.

## Estado contrastado

La inspección de rutas, DTO, repositorios y migración confirma registro por
email/contraseña, enlace de verificación y reenvío; verificar no inicia sesión.
Existe página GET HTML: #14 no describe completamente el estado actual.
`REGISTRATION.md` conserva texto antiguo sobre rate limiting y ausencia de
gateway: el código actual ya incluye protección Redis y configuración Nginx.
No se corrige ese documento fuera del alcance; no usarlo como prueba del estado.
Hay tablas de sesión/refresh y función de reemplazo, sin lógica HTTP de sesión,
firma, JWKS, outbox ni integración distribuida. Redis actual no es autoridad
de sesiones. Las propuestas desktop de 30 días deslizantes y fallback a memoria
son históricas y quedan sustituidas por D-16, sin modificar clientes.

## Secuencia recomendada

| Entrega posterior | Dependencias y alcance | Evidencia exigida |
|---|---|---|
| Revisión de contratos | Equipo Auth, Core, Files y clientes revisa ADR y propuestas; PR documental separado antes de implementar | Aprobar D-16/ADR; resolver parámetros operativos y nombres definitivos |
| Seguridad previa al login | #17, política de enumeración y recuperación segura; verificar T-04/usuario DB propio y límites de intentos fallidos | Dueño recupera correo sin conservar contraseña de atacante; carreras y enumeración probadas |
| Auth: claves y JWT | Perfil aprobado; dependencias criptográficas elegidas en entrega autorizada | Firma/validación Rust, JWKS sin privada, rotación/fallos; ningún secreto versionado |
| Auth: sesiones y refresh | Seguridad previa resuelta; transacciones y evolución SQL/outbox revisadas | Login/reemplazo, 20 días absolutos, replay revoca y commit de outbox; rollback sin efectos parciales |
| Auth: API privada y worker | Credenciales/backend, HTTPS privado, límites, outbox y broker TLS | Contratos individual/lote, cuotas independientes, confirmación/reintentos y recuperación |
| Clientes | Login/refresh/logout implementados; llaveros evaluados por SO | Coordinación entre tareas/procesos, restauración, guardado del sucesor y fallos sin secretos |
| Core y Files | Sus responsables avancen; API privada y eventos disponibles | Verificación local, microcaché, difusión, WS y transferencias con deadlines |
| PoC e integración | Auth y Core disponibles; coordinar con responsables | Rust firma, Elixir verifica Ed25519/JWKS con fixtures comunes; CA-03 de #35 sigue pendiente |

La siguiente entrega recomendada es resolver el flujo seguro de #17 y su política
de enumeración antes de habilitar login; después, claves/JWKS y lógica transaccional
de Auth. No inicializar Core/Files para desbloquear artificialmente la PoC.

## Matriz de pruebas posteriores

- JWT: firma modificada, algoritmo/curva/typ incorrectos, issuer/audiencia,
  kid desconocido, tipos/duplicados, iat futuro, exp exacto, access cerca del día 20;
  JWKS 300 s, rotación/solapamiento, fallo de actualización y ausencia de privada.
- Sesiones: dos logins simultáneos mismo tipo, independencia desktop/mobile,
  sesión expirada sin limpiar, refresh simultáneo, replay antiguo tras varias
  rotaciones, carrera logout/refresh, reemplazo/refresh, cuenta suspendida,
  fallo antes/después del commit y retención de hashes consumidos.
- API privada: falta de credencial, credencial de usuario rechazada, servicio sin
  permiso, cuenta/sid distintos, lotes duplicados y límites de tamaño/concurrencia,
  fallo parcial del lote, PostgreSQL caído, certificado incorrecto y timeout.
- Microcaché con reloj controlado: inicio t0, latencia descontada, acierto no
  deslizante, deadline exacto, JWT nuevo para sid cacheado pero inválido, sub
  distinto, evento antes/durante/después de consulta, tombstone y respuesta tardía,
  Auth caído con positivo vigente/vencido, breaker y recuperación sin stale fallback.
- RabbitMQ: caída tras commit, antes/después de confirm, duplicados/desorden,
  no enrutados, DLQ, reconexión; probar todos los caminos SQL de revocación.
  Core multinodo: consumidores competidores más PubSub, nodos desconectados,
  reinicio y cache vacía; Files debe acordar y probar su difusión explícita.
- WS: conectar, acción protegida, entrega, comprobación periódica, timer retrasado,
  expiración JWT/sesión y evento de revocación. Error de conexión ya abierta por
  protocolo; ninguna acción/entrega tras deadline. No afirmar HTTP 503 en socket.
- Clientes: inicio automático, cierre app conserva, logout confirmado borra,
  503 conserva refresh, respuesta incierta exige login, fallo al escribir llavero,
  sin respaldo en claro, sin tokens en logs. Transferencias largas: pruebas
  pendientes de política Files, incluyendo reanudación y URLs firmadas.

## Pendientes explícitos

Llavero no disponible y fallo al guardar sucesor; UX de logout sin conexión;
issuer/audiencias definitivos; formato/hash refresh propuestos; tolerancia de
reloj, margen de rotación y compromiso de claves; distribución/rotación de
credenciales de servicio; timeout, cuotas, lote, breaker y backoff propuestos;
protocolo WS de reautenticación/errores; difusión recuperable Core; estrategia
multinstancia Files y transferencias; retención/limpieza outbox y tombstones;
soporte real Ed25519 Rust–Elixir. Ninguno se considera probado por esta entrega.

## Validación de esta entrega

Herramientas sin instalación: Git y Python estándar. Revisar JSON de los OpenAPI
(sintaxis JSON compatible con YAML), referencias internas, estructura básica de
operaciones/respuestas/security, schema del evento y ejemplo, motivos contra SQL,
enlaces relativos y diff. No están instalados PyYAML, jsonschema ni
openapi-spec-validator: el chequeo estructural local no es certificación completa
OpenAPI/JSON Schema. No se ejecuta sistema, infraestructura ni pruebas de servicios;
no demuestra interoperabilidad. CA-01 requiere aprobación, CA-02 revisión del
contrato, CA-03 PoC pendiente. No se cierra ni modifica ningún issue.

Resultado local: ambos OpenAPI parseados; seis operaciones y referencias locales
resueltas; 44 enlaces relativos comprobados; ejemplo de evento contrastado con
campos requeridos, constantes, enums, UUID y fechas de su esquema; seis motivos
idénticos al CHECK SQL. `git diff --check` sin errores. Chequeos de Python ad hoc
con biblioteca estándar, sin añadir herramientas al repositorio.
