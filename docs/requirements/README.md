# Requisitos

Especificación funcional de Sonimbus. Es la versión **canónica** dentro del repositorio: si un documento externo (la entrega AR02, el traspaso u otros) no coincide con esta carpeta, manda esta carpeta, y el cambio se registra abajo.

| Documento | Contenido |
|---|---|
| [`casos-de-uso.md`](casos-de-uso.md) | Los 33 casos de uso con flujos, alternativas y excepciones |
| [`requisitos.md`](requisitos.md) | Requisitos funcionales, no funcionales y restricciones, con su módulo |

Los requisitos nuevos siguen el formato de la sección 6 de [`STANDARD_DEVELOPMENT.md`](../STANDARD_DEVELOPMENT.md) (`RF-<MÓDULO>-<NN>`, con criterios de aceptación `CA-NN`).

## Alcance del MVP

| Área | MVP | Fase posterior |
|---|---|---|
| Identidad y acceso | Registro, verificación de cuenta, inicio de sesión con correo o Google, cierre de sesión, recuperación de contraseña y perfil | — |
| Grupos y permisos | Creación, configuración, archivado y eliminación de grupos; invitaciones; unirse y abandonar; canales; roles y permisos; moderación | — |
| Mensajería y archivos | Mensajes en tiempo real sin duplicados, respuestas, reacciones, edición y eliminación, historial paginado, archivos e imágenes, archivos compartidos | Traducción de mensajes |
| Proyectos y tareas | Proyectos, tareas, asignación y estado | — |
| Voz | Canales de voz con control de micrófono (D-01) | Transcripción y resumen de reuniones |
| Reportes | Reportes básicos de actividad; exportación a PDF o CSV (prioridad baja) | — |
| Notificaciones | Notificaciones internas en tiempo real | Notificaciones push en móvil |
| Clientes | Escritorio (JavaFX) y móvil (Flutter) | — |
| Fuera del alcance | Cliente web y videollamadas | — |

## Trazabilidad por módulo

Contexto mínimo para trabajar cada módulo de forma independiente. Los contratos (`contracts/`) son el punto de acuerdo entre módulos.

### Auth Service

- **Responsable:** @tuzc0 · [`services/auth/`](../../services/auth/)
- **Casos de uso:** CU-01 a CU-07.
- **Requisitos:** RF-AUTH-01 a 07 · RNF-SEG-01, 02, 04, 05 · RNF-DIS-02 · RES-04, RES-10.
- **Decisiones:** D-06 (perfil y foto predefinida), D-07 (EdDSA), D-11 (Argon2id), D-12 (una sesión por tipo de cliente), D-13 (esquema `auth`).
- **Produce:** `contracts/openapi/auth.yaml`, la clave pública para verificar tokens y el evento `user.updated`.

### Core Service

- **Responsable:** @Maple-M136279841 · [`services/core/`](../../services/core/)
- **Casos de uso:** CU-08 a CU-20, CU-22 (plano de control de la voz), CU-25 a CU-28 y CU-31. Participa en CU-23 publicando el mensaje del archivo.
- **Requisitos:** RF-GRP-01 a 08 · RF-MSG-01, 02, 03, 05, 06, 07 · RF-PRJ-01, 02 · RF-NOT-01 · RF-VOZ-01, 02 · RNF-SEG-02, 03, 04 · RNF-REN-01 a 03 · RNF-CON-01, 02 · RNF-DIS-01, 02.
- **Decisiones:** D-01, D-04, D-05 (ScyllaDB), D-08 (orden en el servidor), D-13 (esquema `core`), D-15 (token de LiveKit).
- **Escenarios de concurrencia:** del 1 al 8 ([`docs/architecture/`](../architecture/README.md#escenarios-de-concurrencia)).
- **Produce:** `contracts/openapi/core.yaml`, los canales y eventos WebSocket, `checkAccess` (gRPC) y eventos de dominio en RabbitMQ.

### File Service

- **Responsable:** @Maple-M136279841 · [`services/files/`](../../services/files/)
- **Casos de uso:** CU-23 y CU-24.
- **Requisitos:** RF-MSG-04, RF-MSG-08 · RNF-DIS-01 · RNF-SEG-02, 03.
- **Decisiones:** D-03 (Elixir), D-13 (esquema `files`). La transferencia (gRPC o REST) está pendiente.
- **Consume:** `checkAccess` de Core para decidir quién puede ver un archivo.

### Report Service

- **Responsable:** @roluva · [`services/reports/`](../../services/reports/)
- **Casos de uso:** CU-29 y CU-30.
- **Requisitos:** RF-REP-01, RF-REP-02 · RNF-DIS-01 · RNF-SEG-03.
- **Decisiones:** D-04 (consume eventos de RabbitMQ), D-13 (esquema `reports`). El lenguaje está pendiente (propuesta: Elixir).
- **Consume:** eventos `message.created`, `task.updated` y de membresía para calcular agregados; `checkAccess` para verificar que quien consulta es administrador.

### Cliente de escritorio

- **Responsable:** @Maple-M136279841 · [`clients/desktop/`](../../clients/desktop/)
- **Casos de uso:** todos los del MVP. Concentra la administración detallada (RES-06).
- **Requisitos:** RNF-CON-01 (identificador del mensaje y cola local), RNF-CON-03, RNF-USA-01, 02, RNF-POR-02, RNF-REN-02, 03.
- **Consume:** `contracts/openapi/`, `contracts/events/` y LiveKit por WebRTC.

### Cliente móvil

- **Responsable:** @roluva · [`clients/mobile/`](../../clients/mobile/)
- **Prioridad (RES-05), propuesta a confirmar por el responsable:**
  - **Incluidos:** CU-01 a CU-07, CU-11, CU-12, CU-16 a CU-20, CU-22, CU-23, CU-24, CU-26, CU-28 y CU-31.
  - **Solo consulta:** proyectos (CU-25) y reportes (CU-29).
  - **Solo en escritorio (RES-06):** CU-09, CU-10, CU-13, CU-14, CU-15, CU-27 y CU-30.
- **Requisitos:** RNF-CON-01, 03, RNF-USA-01, 02, RNF-REN-02, 03.
- **Consume:** los mismos contratos que escritorio (RES-03) y el SDK oficial de LiveKit para Flutter.

### API Gateway e infraestructura

- **Responsable:** @tuzc0 · [`gateway/nginx/`](../../gateway/nginx/), [`infrastructure/`](../../infrastructure/)
- **Requisitos:** RNF-POR-01, RNF-DIS-02, RNF-SEG-02 (TLS y límite de peticiones), RES-07.
- **Decisiones:** D-02, D-04, D-05, D-09, D-13, D-14.

## Cambios respecto a la AR02

La AR02 (entregada el 20 sep 2026) se revisó el 30 sep y se ajustó con las decisiones del 2026-10-07. Cambios que modifican el comportamiento:

| Caso o requisito | AR02 | Versión canónica | Origen |
|---|---|---|---|
| CU-22, RF-VOZ-01, 02 | Voz condicionada a validación técnica | Voz dentro del MVP | D-01 |
| RNF-INT-02 | Broker opcional | RabbitMQ dentro del MVP | D-04 |
| CU-07 Foto de perfil | Paquete predefinido | Paquete predefinido (se descarta la subida de imagen propia de la revisión del 30 sep) | Equipo, 2026-10-07 |
| CU-10 Invitaciones | Administrador y moderador | Administrador y moderador (la revisión del 30 sep lo limitaba al administrador) | Equipo, 2026-10-07 |
| CU-21 Traducir | Automática, según el idioma del sistema operativo | A petición del miembro, eligiendo el idioma | Revisión 30 sep; equipo, 2026-10-07 |
| CU-25 Proyectos | Lo gestiona el administrador; cualquier miembro consulta | Igual, más el aviso al archivar con tareas abiertas | Unión de ambas versiones |
| CU-26 Tareas | Crear y modificar | Crear, modificar y **eliminar** (creador o administrador) | Revisión 30 sep |
| CU-27 Asignar tarea | Cualquier miembro con permiso | **Solo el administrador** | Revisión 30 sep; equipo, 2026-10-07 |
| CU-28 Estado de tarea | Cualquier miembro con permiso | **El responsable o el administrador**; se puede reabrir; se notifica al creador | Revisión 30 sep; equipo, 2026-10-07 |
| Estados de tarea | Pendiente, en progreso, terminada | Pendiente, En progreso, **Completada** | Revisión 30 sep |

Precisiones que no cambian el comportamiento, pero que conviene conocer al implementar:

- **Excepciones generales EX-G1 a EX-G4:** se definen una sola vez en lugar de repetirse en cada caso.
- **CU-11:** se puede unir mediante enlace, incluso sin sesión (FA-01), y descontar el uso es atómico (escenario de concurrencia 4).
- **CU-14:** al cambiar un rol se invalida la caché de permisos (escenario 6).
- **CU-15 y CU-22:** expulsar o bloquear a alguien lo desconecta de la voz.
- **CU-16:** el identificador del mensaje se llama `client_message_id` y el orden lo asigna el servidor (D-08).
- **CU-23:** el cliente valida antes de enviar, puede dejar el archivo pendiente si no hay conexión y las subidas incompletas se limpian periódicamente.
- **CU-29:** se valida un rango máximo del periodo y se muestran gráficas y tabla.
- **CU-31:** la actividad en canales de voz y la moderación generan notificaciones, y se pueden marcar todas como leídas.
