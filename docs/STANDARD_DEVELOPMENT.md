# Estándar de Desarrollo del Proyecto

**Versión:** 0.3  
**Estado:** Vigente  
**Proyecto:** Sonimbus, plataforma de comunicación y gestión ligera de proyectos  
**Clientes:** Aplicación de escritorio y aplicación móvil  
**Infraestructura:** Ubuntu Server 24.04 LTS, Vagrant, VirtualBox, Docker Engine y Docker Compose

> Las decisiones técnicas (`D-NN`) se registran en [`docs/decisions/`](decisions/README.md). Si este estándar y el registro no coinciden, manda el registro y se corrige este documento.

---

## 1. Objetivo

Este documento define las normas técnicas, prácticas de desarrollo y criterios mínimos que deberán seguir los integrantes del equipo durante el desarrollo del sistema.

Su finalidad es garantizar:

- consistencia entre módulos;
- calidad del código;
- seguridad;
- trazabilidad de cambios;
- facilidad de mantenimiento;
- integración entre componentes;
- reducción de errores;
- preparación para una arquitectura distribuida.

Este documento deberá actualizarse conforme se incorporen nuevos módulos, tecnologías o necesidades.

---

## 2. Arquitectura general

Arquitectura distribuida orientada a servicios. El detalle de componentes, comunicación, propiedad de datos y concurrencia está en [`docs/architecture/`](architecture/README.md).

| Componente | Tecnologías | Responsabilidad |
|---|---|---|
| Aplicación de escritorio | Java 21, JavaFX 21, Maven, SQLite, `webrtc-java` | Interfaz, sesión local, cola de pendientes, voz. Concentra la administración detallada (RES-06). |
| Aplicación móvil | Flutter, Dart, Drift (SQLite), SDK de LiveKit | Funciones principales con prioridad en comunicación y consulta (RES-05). |
| API Gateway | Nginx | TLS, enrutamiento REST, WebSocket y gRPC, límite de peticiones. No valida tokens (D-02). |
| Auth Service | Rust, Axum, Tokio, Tower, SQLx | Registro, verificación, inicio de sesión con correo y Google, sesiones, tokens EdDSA, recuperación de contraseña y **perfil de usuario** (D-06). |
| Core Service | Elixir, Phoenix, Erlang/OTP | Grupos, membresías, invitaciones, canales, roles, permisos y moderación; mensajería, reacciones e historial; proyectos y tareas; notificaciones; plano de control de la voz (D-15). |
| File Service | Elixir | Recepción, validación, almacenamiento, vistas previas y entrega de archivos (D-03). |
| Report Service | Por decidir (propuesta: Elixir) | Agregados de actividad y reportes. |
| Servidor de medios | LiveKit autoalojado | Audio de los canales de voz (D-01). |
| Broker | RabbitMQ | Eventos asíncronos entre servicios (D-04). |

Ambos clientes consumen los mismos servicios (RES-03).

Los dominios de Core deben mantenerse desacoplados para facilitar una posible separación futura en servicios independientes.

---

## 3. Persistencia de datos

Se utilizará persistencia políglota. Cada servicio es dueño de sus datos y ningún servicio lee los datos de otro. El modelo detallado está en [`docs/database/`](database/README.md).

### 3.1 PostgreSQL

Información estructurada y fuertemente relacionada: cuentas, sesiones y perfil (Auth); grupos, miembros, invitaciones, canales, roles, permisos, proyectos, tareas y notificaciones (Core); metadatos de archivos (Files); agregados de actividad (Reports).

Una sola instancia y una sola base de datos, con **un esquema y un usuario por servicio** (`auth`, `core`, `files`, `reports`) (D-13).

### 3.2 ScyllaDB

Base NoSQL distribuida para los mensajes y las reacciones, de gran volumen y escritura intensa.

ScyllaDB es la base NoSQL seleccionada oficialmente para el proyecto (D-05). Es compatible con CQL y con los drivers de Cassandra.

### 3.3 Redis

Redis no será el almacenamiento principal. Se utilizará para:

- sesiones e intentos fallidos (Auth);
- presencia y usuarios conectados (Core);
- caché de permisos (Core);
- rate limiting e información temporal.

### 3.4 MinIO

Los archivos binarios (imágenes, documentos y adjuntos) se almacenarán en almacenamiento de objetos, administrado por el File Service.

Las bases de datos almacenarán únicamente los metadatos y la referencia correspondiente.

---

## 4. Infraestructura

El sistema será desplegado inicialmente en una máquina virtual con **Ubuntu Server 24.04 LTS**.

El entorno local se administrará mediante Vagrant y VirtualBox. La configuración común de desarrollo será:

```text
RAM:        6 GB
CPU:        4 CPU virtuales
Disco:      80 GB
IP privada: 192.168.33.30
```

El `Vagrantfile` deberá definir la máquina y `scripts/provision.sh` deberá instalar Docker Engine y Docker Compose de forma reproducible e idempotente.

Durante las primeras etapas, los componentes del servidor se ejecutarán mediante Docker Compose:

```text
Equipo anfitrión
│
└── Vagrant + VirtualBox
    │
    └── Ubuntu Server 24.04 LTS
        │
        └── Docker Compose
            ├── Servicios predeterminados
            │   ├── postgres
            │   ├── redis
            │   └── mailpit
            ├── Perfil messaging
            │   └── cassandra (se reemplaza por scylladb, T-01)
            ├── Perfil storage
            │   └── minio
            └── Pendientes
                ├── rabbitmq
                ├── livekit
                └── nginx
```

Mailpit será utilizado exclusivamente en desarrollo para probar recuperación de contraseña, verificación de correo y otras funciones relacionadas con email.

El gateway, los servicios (Auth, Core, Files y Reports), RabbitMQ y LiveKit se incorporarán a Compose cuando exista su implementación inicial.

Para la expo, el mismo Compose corre en Ubuntu nativo (D-10, D-14); ver [`docs/deployment/`](deployment/README.md).

Los datos persistentes deberán almacenarse en volúmenes nombrados administrados por Docker. No deberán crearse carpetas de datos versionadas dentro del repositorio.

Los puertos publicados se utilizarán únicamente a través de la red privada de desarrollo. En despliegues posteriores, las bases de datos no deberán exponerse directamente a redes externas.

La configuración sensible se almacenará en `infrastructure/.env`. Este archivo no deberá versionarse; el repositorio conservará únicamente `infrastructure/.env.example` con valores de referencia.

---

## 5. Organización del repositorio

```text
team-collaboration-platform/
├── .github/            plantillas, CODEOWNERS y CI
├── clients/
│   ├── desktop/        Java 21 + JavaFX (app/ y livekit-sdk/)
│   └── mobile/         Flutter + Drift
├── services/
│   ├── auth/           Rust: identidad, perfil y tokens
│   ├── core/           Elixir/Phoenix: dominio principal
│   ├── files/          Elixir: archivos sobre MinIO
│   └── reports/        reportes de actividad
├── gateway/
│   └── nginx/          API gateway
├── contracts/
│   ├── openapi/        contratos REST por servicio
│   ├── events/         eventos WebSocket y RabbitMQ
│   └── proto/          contratos gRPC (al crearse)
├── database/
│   ├── postgres/       inicialización de la instancia
│   └── scylla/         esquemas CQL
├── infrastructure/     Compose, .env.example y LiveKit
├── scripts/
├── docs/
│   ├── PLAN.md         equipo, reparto, fases y tareas
│   ├── architecture/
│   ├── requirements/   casos de uso y requisitos canónicos
│   ├── database/
│   ├── deployment/
│   └── decisions/      registro de decisiones y ADR
├── Vagrantfile
└── README.md
```

Cada carpeta de `clients/`, `services/` y `gateway/` es un módulo con un responsable principal, indicado en su `README.md` junto con su responsabilidad, su stack y lo que queda fuera de su alcance. El reparto vigente está en [`docs/PLAN.md`](PLAN.md#reparto-de-módulos).

### Alcances

El nombre del módulo se usa como alcance en los commits y como `<modulo>` en las ramas:

| Ruta | Alcance |
|---|---|
| `clients/desktop/` | `desktop` |
| `clients/mobile/` | `mobile` |
| `services/auth/` | `auth` |
| `services/core/` | `core` |
| `services/files/` | `files` |
| `services/reports/` | `reports` |
| `gateway/` | `gateway` |
| `contracts/` | `contracts` |
| `database/` | `database` |
| `infrastructure/`, `Vagrantfile`, `scripts/` | `infrastructure` |
| `docs/` | `docs` |

### Reglas

- Un PR modifica un solo módulo, salvo cambios de contrato acompañados de su implementación.
- Los clientes y servicios se coordinan mediante `contracts/`, no leyendo el código del otro.
- Cada servicio es dueño de sus datos; las migraciones viven dentro del servicio.

La organización podrá modificarse si el crecimiento del proyecto lo justifica.

---

## 6. Gestión de requisitos

Los requisitos deberán definirse por módulo.

Cada requisito funcional deberá incluir:

- identificador;
- nombre;
- descripción;
- prioridad;
- criterios de aceptación;
- módulo asociado.

Ejemplo:

```text
RF-AUTH-01

Nombre:
Inicio de sesión.

Descripción:
El sistema deberá permitir que un usuario registrado
inicie sesión mediante sus credenciales.

Prioridad:
Alta.

Criterios de aceptación:

CA-01:
Si las credenciales son válidas, el sistema deberá permitir el acceso.

CA-02:
Si las credenciales son inválidas, el sistema deberá rechazar el acceso.

CA-03:
La contraseña no deberá almacenarse ni registrarse en texto plano.
```

Los requisitos deberán tomar como referencia ISO/IEC/IEEE 29148.

---

## 7. Requisitos no funcionales

Se establecen inicialmente:

### Seguridad
Protección de información sensible, credenciales y recursos mediante mecanismos apropiados de autenticación, autorización y cifrado.

### Rendimiento
Las operaciones críticas deberán establecer tiempos de respuesta medibles.

### Disponibilidad
La arquitectura deberá permitir detectar y gestionar fallos parciales.

### Escalabilidad
Los componentes deberán diseñarse evitando dependencias que impidan incrementar capacidad posteriormente.

### Compatibilidad
Los clientes deberán establecer claramente plataformas y versiones soportadas.

### Accesibilidad
Las interfaces deberán incorporar progresivamente criterios de accesibilidad.

Como referencia general de calidad se utilizará ISO/IEC 25010.

---

## 8. Gestión de Git

La rama principal será:

```text
main
```

`main` deberá mantenerse funcional.

No se deberán realizar cambios directamente sobre `main`, salvo situaciones excepcionales autorizadas.

Las funcionalidades deberán desarrollarse en ramas independientes.

Formato:

```text
feature/<modulo>-<descripcion>
fix/<modulo>-<descripcion>
tech/<descripcion>
refactor/<modulo>-<descripcion>
docs/<descripcion>
test/<modulo>-<descripcion>
```

Ejemplos:

```text
feature/auth-login
feature/core-groups-create
fix/auth-token-expiration
tech/docker-compose
tech/linux-vm
tech/postgresql-container
tech/scylladb-setup
refactor/core-messages-repository
docs/system-architecture
test/auth-login
```

---

## 9. Convención de commits

Se utilizará Conventional Commits.

Tipos iniciales:

```text
feat
fix
docs
test
refactor
chore
build
ci
perf
```

Formato:

```text
tipo(alcance): descripción
```

Ejemplos:

```text
feat(auth): add user login
feat(core): add group creation
fix(auth): reject expired refresh tokens
test(core): add websocket integration tests
docs(contracts): document authentication endpoints
refactor(core): separate project repository
```

Evitar mensajes como:

```text
cambios
ya funciona
final
final2
correccion
aaa
```

---

## 10. Pull Requests

Todo cambio relevante deberá incorporarse mediante Pull Request.

Una Pull Request deberá incluir:

- descripción;
- issue o requisito relacionado;
- pruebas realizadas;
- evidencia cuando corresponda;
- riesgos conocidos.

Flujo:

```text
Código
  ↓
Pruebas
  ↓
Pull Request
  ↓
Code Review
  ↓
CI
  ↓
Merge
```

---

## 11. Code Review

Durante una revisión deberán comprobarse:

### Funcionalidad
- ¿Cumple el requisito?
- ¿Maneja casos de error?

### Diseño
- ¿La responsabilidad pertenece a ese módulo?
- ¿Existe acoplamiento innecesario?

### Código
- ¿Los nombres son comprensibles?
- ¿Existen duplicaciones importantes?
- ¿Puede simplificarse?

### Seguridad
- ¿Se valida la entrada?
- ¿Existen datos sensibles expuestos?
- ¿Se verifican permisos?
- ¿Se utilizan secretos correctamente?

### Pruebas
- ¿La nueva lógica posee pruebas?
- ¿Las pruebas validan comportamiento relevante?

---

## 12. Definition of Done

Una tarea se considerará terminada cuando, cuando corresponda:

- el requisito esté implementado;
- los criterios de aceptación estén cumplidos;
- las pruebas unitarias sean exitosas;
- las pruebas de integración sean exitosas;
- el code review esté completado;
- el CI sea exitoso;
- la documentación esté actualizada;
- la API esté actualizada;
- el build sea exitoso;
- la imagen Docker construya correctamente;
- no existan secretos dentro del código;
- no existan vulnerabilidades críticas conocidas introducidas por el cambio.

"Funciona en mi computadora" no constituye un criterio de finalización.

---

## 13. Diseño de API REST

Las APIs deberán seguir una estructura consistente.

```text
GET    /projects
GET    /projects/{id}
POST   /projects
PATCH  /projects/{id}
DELETE /projects/{id}
```

Códigos de respuesta:

```text
200 OK
201 Created
204 No Content
400 Bad Request
401 Unauthorized
403 Forbidden
404 Not Found
409 Conflict
422 Unprocessable Content
500 Internal Server Error
```

Las APIs REST deberán documentarse mediante OpenAPI.

---

## 14. Manejo de errores

Los servicios deberán devolver errores mediante una estructura común.

Ejemplo conceptual:

```json
{
  "type": "/problems/invalid-credentials",
  "title": "Invalid credentials",
  "status": 401,
  "detail": "The supplied credentials are invalid."
}
```

Los errores internos no deberán revelar:

- stack traces;
- contraseñas;
- tokens;
- consultas SQL;
- rutas internas;
- secretos;
- detalles innecesarios de infraestructura.

---

## 15. Eventos WebSocket

Formato recomendado:

```text
<recurso>.<evento>
```

Ejemplos:

```text
message.created
message.updated
message.deleted
reaction.added
user.joined
user.left
meeting.started
meeting.finished
```

Los eventos deberán incluir versión cuando exista riesgo de cambios incompatibles.

```json
{
  "type": "message.created",
  "version": 1,
  "payload": {}
}
```

---

## 16. Comunicación entre servicios

Los clientes utilizarán:

- REST, a través del gateway;
- WebSocket (canales de Phoenix) para tiempo real;
- WebRTC hacia LiveKit para la voz.

La comunicación interna entre servicios utilizará:

- gRPC para consultas síncronas, como `checkAccess` de Core;
- AMQP (RabbitMQ) para eventos asíncronos, como `message.created` o `user.updated` (D-04).

No deberá añadirse un mecanismo de comunicación únicamente para demostrar el uso de una tecnología.

---

## 17. Seguridad

La seguridad deberá formar parte del desarrollo desde el inicio.

Referencias:

- OWASP ASVS;
- OWASP MASVS;
- OWASP API Security;
- NIST SSDF;
- ISO/IEC 27001 y 27002 como referencias generales.

### Contraseñas

Nunca deberán almacenarse en texto plano.

Se propone Argon2id.

### Secrets

No deberán incorporarse al repositorio:

```text
passwords
API keys
private keys
JWT secrets
database credentials
```

Se utilizarán variables de entorno o mecanismos de secrets.

---

## 18. Autenticación y autorización

### Autenticación

Determina:

```text
¿Quién es el usuario?
```

Responsabilidad principal: Auth Service.

### Autorización

Determina:

```text
¿Qué puede hacer el usuario?
```

La posesión de un token válido no será suficiente para acceder a cualquier recurso.

---

## 19. Logging

Los servicios deberán generar logs estructurados.

Campos recomendados:

```text
timestamp
service
level
request_id
user_id
event
```

No registrar:

- contraseñas;
- access tokens;
- refresh tokens;
- secret keys;
- datos sensibles innecesarios.

Niveles:

```text
DEBUG
INFO
WARN
ERROR
```

---

## 20. Observabilidad

Se incorporarán progresivamente:

- logs;
- métricas;
- tracing.

Se utilizará OpenTelemetry como referencia cuando se implemente tracing distribuido.

---

## 21. Pruebas

Se utilizarán:

- pruebas unitarias;
- pruebas de integración;
- pruebas de contrato;
- pruebas end-to-end.

Ejemplo E2E:

```text
Registro
   ↓
Login
   ↓
Crear grupo
   ↓
Enviar mensaje
```

---

## 22. Cobertura

La cobertura será un indicador, no el único criterio de calidad.

Se respetarán los mínimos académicos definidos para el proyecto.

No deberán escribirse pruebas inútiles únicamente para incrementar el porcentaje.

---

## 23. CI/CD

Toda Pull Request deberá ejecutar progresivamente:

```text
Checkout
   ↓
Build
   ↓
Lint
   ↓
Unit Tests
   ↓
Integration Tests
   ↓
Security Checks
   ↓
Docker Build
```

Posteriormente:

```text
main
 ↓
Docker build
 ↓
Container registry
 ↓
VM Linux
 ↓
Deployment
```

---

## 24. Docker

Cada servicio deberá tener su propio Dockerfile cuando corresponda.

Buenas prácticas:

- utilizar imágenes oficiales;
- fijar versiones relevantes;
- minimizar tamaño;
- evitar root cuando sea posible;
- utilizar `.dockerignore`;
- no incluir secretos;
- utilizar health checks cuando sea necesario;
- separar configuración del código.

---

## 25. Docker Compose

Docker Compose será utilizado inicialmente para levantar la infraestructura local y, posteriormente, los servicios del backend.

Servicios predeterminados:

```text
postgres
redis
mailpit
```

Servicios opcionales mediante perfiles:

```text
messaging:
  scylladb   (hoy cassandra; cambio pendiente T-01)

storage:
  minio
```

Los perfiles deberán permitir que cada integrante inicie únicamente los componentes necesarios para su tarea y reduzca el consumo de recursos.

Cuando exista su implementación inicial, se añadirán:

```text
gateway
auth-service
core-service
files-service
reports-service
rabbitmq
livekit
```

Deberá existir una red interna para la comunicación entre contenedores. Los puertos publicados, las credenciales y las opciones locales deberán configurarse mediante variables de entorno.

---

## 26. Persistencia

Los servicios con información persistente deberán utilizar volúmenes.

Ejemplos:

```text
postgres_data
redis_data
scylla_data
minio_data
```

Recrear un contenedor no deberá implicar perder información persistente.

---

## 27. Accesibilidad

Se utilizarán como referencia principios de WCAG 2.2 cuando sean aplicables:

- contraste;
- navegación mediante teclado;
- lectores de pantalla;
- escalado de texto;
- labels descriptivos;
- no depender únicamente del color;
- subtítulos y transcripciones cuando existan funciones audiovisuales.

---

## 28. Estilo de código

### Rust

```text
rustfmt
clippy
```

### Elixir

```text
mix format
Credo
```

### Dart / Flutter

```text
dart format
flutter analyze
```

### Java

Se deberá definir un formateador común para todo el equipo.

El formato automático tendrá prioridad sobre preferencias personales.

---

## 29. Dependencias

Toda dependencia deberá:

- tener una finalidad identificable;
- estar activamente mantenida cuando sea posible;
- poseer licencia compatible;
- evitar vulnerabilidades críticas conocidas;
- añadirse solo cuando aporte valor real.

---

## 30. Documentación

El repositorio deberá mantener como mínimo:

```text
README.md

docs/
├── PLAN.md
├── architecture/
├── requirements/
├── database/
├── deployment/
└── decisions/

contracts/            documentación de API (OpenAPI, eventos y gRPC)
```

---

## 31. Architecture Decision Records

Las decisiones técnicas se registran en [`docs/decisions/README.md`](decisions/README.md) con un identificador `D-NN`. Las que cambian la arquitectura, tienen alternativas serias o son costosas de revertir se desarrollan además como ADR en la misma carpeta (`ADR-NNN-titulo.md`).

Formato de un ADR:

```text
Título
Estado
Fecha
Contexto
Decisión
Alternativas consideradas
Consecuencias
```

---

## 32. Versionado

Se utilizará Semantic Versioning cuando existan releases formales.

```text
MAJOR.MINOR.PATCH
```

Ejemplos:

```text
0.1.0
0.2.0
1.0.0
1.0.1
```

---

## 33. Forma de trabajo

```text
Requisito
   ↓
Issue
   ↓
Diseño
   ↓
Branch
   ↓
Implementación
   ↓
Pruebas
   ↓
Pull Request
   ↓
Code Review
   ↓
CI
   ↓
Merge
   ↓
Deploy de integración
```

---

## 34. Primer módulo

Durante el módulo inicial de autenticación deberán aplicarse por primera vez:

- requisitos identificados;
- criterios de aceptación;
- ramas Git;
- Conventional Commits;
- Pull Requests;
- Code Review;
- Auth Service en Rust;
- PostgreSQL en Docker;
- Redis para información temporal y sesiones cuando corresponda;
- Mailpit para pruebas locales de correo y recuperación de contraseña;
- variables de entorno;
- hashing de contraseña;
- pruebas unitarias;
- documentación de API;
- OpenAPI;
- Dockerfile;
- Docker Compose;
- CI básico.

---

## 35. Principio general

Cuando existan varias soluciones técnicamente correctas, se priorizará:

1. seguridad;
2. claridad;
3. mantenibilidad;
4. capacidad de prueba;
5. simplicidad;
6. desempeño;
7. escalabilidad.

No se deberá incrementar la complejidad únicamente para utilizar una tecnología nueva.

Toda incorporación tecnológica deberá resolver una necesidad identificable.

---

## Estado del documento

**Versión actual:** 0.3 (2026-10-07: alineado con el registro de decisiones, tres integrantes y Report Service).  
**Próxima revisión:** al cerrar el primer recorrido completo (fase 5 de [`docs/PLAN.md`](PLAN.md)).

Este estándar se considera un documento vivo.