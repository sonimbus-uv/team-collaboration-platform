# Plan de trabajo

Equipo, reparto de módulos, fases de implementación y tareas abiertas de Sonimbus. Es la referencia para saber quién hace qué y qué sigue.

- Qué debe hacer el sistema: [`docs/requirements/`](requirements/).
- Cómo está construido: [`docs/architecture/`](architecture/).
- Por qué se decidió así: [`docs/decisions/`](decisions/).

## Equipo

| Integrante | GitHub | CODEOWNERS |
|---|---|---|
| Guillermo Velázquez Rosiles | [@Maple-M136279841](https://github.com/Maple-M136279841) | Sí |
| Claudio Josué Trujillo Zepeda | [@tuzc0](https://github.com/tuzc0) | Sí |
| Rodrigo Luna Vázquez | [@roluva](https://github.com/roluva) | Sí (miembro) |

**Catedrático:** Dr. Saúl Domínguez-Isidro.

| Materia | NRC | Responsabilidad en el proyecto |
|---|---|---|
| Desarrollo de Sistemas en Red | 20226 | Aplicación de servicios (API) y cliente rico de escritorio |
| Despliegue de Software | 20222 | Cliente móvil y despliegue con contenedores sobre GNU/Linux |

## Reparto de módulos

Cada módulo tiene un responsable principal, indicado también en su `README.md`. El responsable decide dentro de su módulo y es quien lo implementa; los cambios de contrato se acuerdan en `contracts/` antes de implementarse.

| Módulo | Ruta | Responsable |
|---|---|---|
| Cliente de escritorio (incluye el fork de LiveKit) | `clients/desktop/` | @Maple-M136279841 |
| Core Service | `services/core/` | @Maple-M136279841 |
| File Service | `services/files/` | @Maple-M136279841 |
| Esquemas CQL de ScyllaDB | `database/scylla/` | @Maple-M136279841 |
| Auth Service | `services/auth/` | @tuzc0 |
| API Gateway | `gateway/nginx/` | @tuzc0 |
| Infraestructura, VM y LiveKit | `infrastructure/`, `Vagrantfile`, `scripts/` | @tuzc0 |
| Inicialización de PostgreSQL | `database/postgres/` | @tuzc0 |
| Cliente móvil | `clients/mobile/` | @roluva |
| Report Service | `services/reports/` | @roluva |
| Contratos | `contracts/` | El dueño de cada servicio escribe el suyo |
| Documentación general | `docs/` | Todo el equipo |

### Revisión

`.github/CODEOWNERS` asigna como revisores a @Maple-M136279841 y @tuzc0 en todo el repositorio, y `main` exige una aprobación. Mientras @roluva no sea codeowner, sus PR los revisa cualquiera de los dos, y él puede revisar PR de otros, aunque su aprobación no cuente para la regla de `main`.

## Fases

Estado al 2026-10-07. ✅ hecho · 🟡 en curso o parcial · ⬜ pendiente.

### Fase 1. Repositorio y organización — 🟡

- ✅ Monorepo con `clients/`, `services/`, `gateway/`, `contracts/`, `database/`, `infrastructure/` y `docs/`.
- ✅ Convención de ramas y Conventional Commits ([`CONTRIBUTING.md`](../CONTRIBUTING.md)).
- ✅ README, `.gitignore` y `.env.example` sin secretos.
- ✅ Documentación de requisitos, arquitectura y decisiones dentro del repositorio.
- 🟡 `services/reports/` existe solo como README (falta decidir su lenguaje).
- ⬜ Tablero de tareas con los casos de uso como *issues* (T-11).
- ⬜ Integración continua con GitHub Actions (T-07).

### Fase 2. Entorno de cada integrante — 🟡

- ⬜ VM de Vagrant funcionando para los tres integrantes (D-14).
- ⬜ Java 21 como predeterminado (no compilar con JDK 24).
- ⬜ Rust, Erlang/Elixir (versiones fijadas en `.tool-versions`) y Flutter instalados según el módulo de cada quien.
- ⬜ Herramientas: `mkcert`, `grpcurl`, `cqlsh`, `psql` y Postman o Bruno.

### Fase 3. Infraestructura en Docker Compose — 🟡 (@tuzc0)

- ✅ PostgreSQL, Redis, Mailpit y MinIO con volúmenes con nombre.
- 🟡 Base de mensajes: hoy es Cassandra; cambiar a ScyllaDB (T-01).
- ⬜ Esquema y usuario por servicio en PostgreSQL (T-04).
- ⬜ *Keyspace* de ScyllaDB creado por script de inicio.
- ⬜ RabbitMQ con plugin de administración y archivo de definiciones (T-03).
- ⬜ *Bucket* de MinIO creado al iniciar.
- ⬜ LiveKit con los puertos validados en la PoC.
- ⬜ Nginx: TLS con `mkcert`, rutas REST, `upgrade` de WebSocket, `limit_req` y `client_max_body_size`.
- 🟡 `healthcheck` en cada contenedor (faltan Mailpit y MinIO).
- ⬜ **Hito:** `docker compose up` deja todo en estado *healthy*.

### Fase 4. Contratos — ⬜

- ⬜ Diagrama entidad-relación de PostgreSQL por servicio.
- ⬜ Tablas de ScyllaDB.
- ⬜ OpenAPI de Auth y Core (`/api/v1`).
- ⬜ Archivos `.proto` (`checkAccess` y, si aplica, transferencia de archivos).
- ⬜ Protocolo de WebSocket: canales de Phoenix y formato de cada evento.
- ⬜ Formato común de errores (Problem Details) y paginación por cursor.
- ⬜ Esquema JSON de cada evento de RabbitMQ.

### Fase 5. Primer recorrido completo — 🟡

CU-01, CU-03, CU-08, CU-11, CU-16 y CU-20.

- 🟡 **Auth** (@tuzc0): ✅ esqueleto, migración inicial y *health checks*; ⬜ registro con Argon2id, verificación por correo, inicio y cierre de sesión, renovación de tokens EdDSA, *endpoint* de clave pública, límite de intentos fallidos.
- ⬜ **Core** (@Maple-M136279841): verificación del JWT, grupos, invitaciones y unirse (cupo atómico), canales de texto, mensajes por WebSocket con idempotencia, historial paginado.
- ⬜ **Escritorio** (@Maple-M136279841): inicio de sesión, lista de grupos, chat en tiempo real, cola local en SQLite, reconexión automática.
- ⬜ **Hito:** dos clientes de escritorio conversan a través de Nginx.

### Fase 6. Resto del MVP — ⬜

- Auth: inicio con Google (CU-04), recuperación de contraseña (CU-05), perfil y evento `user.updated` (CU-07).
- Core: roles, permisos y moderación con invalidación de la caché de permisos (CU-14, CU-15); configurar, archivar, eliminar y abandonar grupo (CU-09, CU-12); invitaciones y canales (CU-10, CU-13); respuestas, reacciones, edición y eliminación (CU-17 a CU-19); proyectos y tareas con control optimista por versión (CU-25 a CU-28); notificaciones internas (CU-31).
- Voz (CU-22): VoiceService emite el token de LiveKit; lista de participantes y estado del micrófono; desconexión al expulsar.
- File Service (CU-23, CU-24): MinIO, vistas previas, estado pendiente con limpieza periódica, listado de archivos compartidos.
- Report Service (CU-29, CU-30): consumo de eventos, agregados y consultas; exportación a PDF y CSV (prioridad baja).

### Fase 7. Cliente móvil — ⬜ (@roluva)

- Flutter con Drift sobre la misma API.
- Funciones priorizadas según RES-05 (ver el alcance del móvil en [`docs/requirements/`](requirements/README.md#cliente-móvil)).
- Voz con el SDK oficial de LiveKit para Flutter.

### Fase 8. Calidad y verificación — ⬜

- Pruebas unitarias de la lógica de negocio y de integración contra la infraestructura real.
- Pruebas de los 8 escenarios de concurrencia ([`docs/architecture/`](architecture/README.md#escenarios-de-concurrencia)).
- Prueba de carga para RNF-REN-01 (menos de 2 s), por ejemplo con k6.
- Logs estructurados sin datos sensibles (RNF-SEG-04).
- Evidencia guardada de cada prueba.

### Fase 9. Despliegue y expo — ⬜

Ver [`docs/deployment/`](deployment/).

### Fase 10. Documentación — 🟡

- 🟡 Documento de diseño con las vistas 4+1 actualizadas a D-01 a D-15.
- ⬜ Reporte detallado de la PoC de voz.
- ⬜ Documentación de la API generada desde OpenAPI.
- ⬜ Documento de despliegue.

## Tareas abiertas

Detectadas en la revisión del repositorio del 2026-10-07. Cada una debe convertirse en *issue* con la plantilla de tarea técnica.

| ID | Tarea | Módulo | Responsable |
|---|---|---|---|
| T-01 | Reemplazar `cassandra:5.0.9` por ScyllaDB en `compose.yaml`, con memoria acotada (`--smp 1 --memory ... --overprovisioned 1`); conservar el perfil `messaging` y renombrar el volumen | `infrastructure` | @tuzc0 |
| T-02 | Alinear valores entre `infrastructure/.env.example` (`team_platform` / `team_platform_user`), los valores por defecto de `compose.yaml` y `services/auth/.env.example` (`team_collaboration` / `team_user`); agregar las variables que faltan (`POSTGRES_PORT`, `REDIS_PORT`, memoria de ScyllaDB) | `infrastructure`, `auth` | @tuzc0 |
| T-03 | Agregar RabbitMQ a `compose.yaml` con archivo de definiciones (*exchange*, colas y cola de mensajes fallidos) | `infrastructure` | @tuzc0 |
| T-04 | Script de inicialización de PostgreSQL: esquemas `auth`, `core`, `files` y `reports` con un usuario por servicio (D-13); Auth deja de conectarse con el superusuario | `database`, `auth` | @tuzc0 |
| T-05 | `healthcheck` de Mailpit y MinIO; creación del *bucket* al iniciar | `infrastructure` | @tuzc0 |
| T-06 | Auth: eliminar los archivos vacíos `src/routes/health.rs` y `src/handlers/` o moverles el código de salud; actualizar la estructura del README | `auth` | @tuzc0 |
| T-07 | CI con GitHub Actions: compilar, formatear y probar cada módulo en cada PR, empezando por Auth | `.github` | Por asignar |
| T-08 | Configurar la identidad de git dentro de la VM (el commit `d20dfd9` quedó firmado, not codeowner como `vagrant <vagrant@Nimbus-Box>`) y documentarlo en el README | `infrastructure` | @tuzc0 |
| T-09 | Verificar que el stack completo (3–4.5 GB estimados más los servicios) cabe en la VM de 6 GB; si no, ajustar perfiles o memoria | `infrastructure` | @tuzc0 |
| T-10 | Inicializar `services/reports/` cuando se decida su lenguaje | `reports` | @roluva |
| T-11 | Crear el tablero de GitHub Projects con un *issue* por caso de uso del MVP | — | Por asignar |
| T-12 | Actualizar el documento de diseño (4+1) con D-01, D-04, D-05 y D-13, que cambian respecto a la AR02 | `docs` | Por asignar |
