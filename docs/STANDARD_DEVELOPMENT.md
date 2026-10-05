# Estándar de Desarrollo del Proyecto

**Versión:** 0.2  
**Estado:** Prototipo  
**Proyecto:** Plataforma de comunicación y gestión ligera de proyectos  
**Clientes:** Aplicación de escritorio y aplicación móvil  
**Infraestructura:** Ubuntu Server 24.04 LTS, Vagrant, VirtualBox, Docker Engine y Docker Compose

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

El sistema estará compuesto inicialmente por dos aplicaciones cliente independientes:

### 2.1 Aplicación de escritorio

Tecnologías previstas:

- Java;
- JavaFX;
- Maven;
- SQLite para almacenamiento local cuando corresponda.

### 2.2 Aplicación móvil

Tecnologías previstas:

- Flutter;
- Dart;
- SQLite mediante Drift para almacenamiento y funcionamiento offline.

Ambos clientes consumirán los mismos servicios proporcionados por el backend.

### 2.3 Auth Service

Tecnologías previstas:

- Rust;
- Axum;
- Tokio;
- Tower;
- SQLx.

Responsabilidades:

- registro;
- inicio de sesión;
- cierre de sesión;
- recuperación de contraseña;
- gestión de sesiones;
- tokens;
- funciones de seguridad asociadas a identidad.

### 2.4 Core Service

Tecnologías previstas:

- Elixir;
- Phoenix;
- Erlang/OTP.

Responsabilidades iniciales:

- usuarios;
- grupos;
- roles;
- permisos;
- proyectos;
- tareas;
- reuniones;
- mensajería;
- reacciones;
- notificaciones;
- lógica principal del sistema.

Los módulos del Core deberán mantenerse desacoplados para facilitar una posible evolución futura hacia microservicios independientes.

---

## 3. Persistencia de datos

Se utilizará persistencia políglota.

### 3.1 PostgreSQL

Se utilizará para información estructurada y fuertemente relacionada:

- usuarios;
- grupos;
- miembros;
- roles;
- permisos;
- proyectos;
- tareas;
- reuniones;
- invitaciones;
- configuración;
- información relacionada con autenticación.

### 3.2 Cassandra

Se utilizará una base NoSQL distribuida para información de gran volumen y acceso orientado a eventos:

- mensajes;
- historial de conversaciones;
- eventos;
- actividad;
- información histórica.

Cassandra es la base NoSQL distribuida seleccionada oficialmente para el proyecto.

### 3.3 Redis

Redis no será el almacenamiento principal. Se utilizará para:

- caché;
- presencia;
- usuarios conectados;
- información temporal;
- rate limiting;
- eventos;
- sesiones cuando sea necesario.

### 3.4 MinIO

Los archivos binarios deberán almacenarse en almacenamiento de objetos:

- imágenes;
- documentos;
- audios;
- videos;
- archivos adjuntos.

Las bases de datos almacenarán únicamente la metadata y referencia correspondiente.

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
            │   └── cassandra
            └── Perfil storage
                └── minio
```

Mailpit será utilizado exclusivamente en desarrollo para probar recuperación de contraseña, verificación de correo y otras funciones relacionadas con email.

El gateway, Auth Service y Core Service se incorporarán a Compose cuando exista su implementación inicial.

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
│   └── files/          Elixir: archivos sobre MinIO
├── gateway/
│   └── nginx/          API gateway
├── contracts/
│   ├── openapi/        contratos REST por servicio
│   └── events/         eventos WebSocket y RabbitMQ
├── database/
│   ├── postgres/       inicialización de la instancia
│   └── scylla/         esquemas CQL
├── infrastructure/     Compose, .env.example y LiveKit
├── scripts/
├── docs/
│   ├── architecture/
│   ├── requirements/
│   ├── database/
│   ├── deployment/
│   └── decisions/      ADR
├── Vagrantfile
└── README.md
```

Cada carpeta de `clients/`, `services/` y `gateway/` es un módulo con un responsable principal, indicado en su `README.md` junto con su responsabilidad, su stack y lo que queda fuera de su alcance.

### Alcances

El nombre del módulo se usa como alcance en los commits y como `<modulo>` en las ramas:

| Ruta | Alcance |
|---|---|
| `clients/desktop/` | `desktop` |
| `clients/mobile/` | `mobile` |
| `services/auth/` | `auth` |
| `services/core/` | `core` |
| `services/files/` | `files` |
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
feature/groups-create
fix/auth-token-expiration
tech/docker-compose
tech/linux-vm
tech/postgresql-container
tech/cassandra-setup
refactor/messages-repository
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
feat(groups): add group creation
fix(auth): reject expired refresh tokens
test(messages): add websocket integration tests
docs(api): document authentication endpoints
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

Los clientes podrán utilizar:

- REST;
- WebSocket.

La comunicación interna entre servicios podrá utilizar:

- gRPC.

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
  cassandra

storage:
  minio
```

Los perfiles deberán permitir que cada integrante inicie únicamente los componentes necesarios para su tarea y reduzca el consumo de recursos.

Cuando exista su implementación inicial, se añadirán:

```text
gateway
auth-service
core-service
```

Deberá existir una red interna para la comunicación entre contenedores. Los puertos publicados, las credenciales y las opciones locales deberán configurarse mediante variables de entorno.

---

## 26. Persistencia

Los servicios con información persistente deberán utilizar volúmenes.

Ejemplos:

```text
postgres_data
redis_data
cassandra_data
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
├── architecture/
├── requirements/
├── api/
├── database/
├── deployment/
└── decisions/
```

---

## 31. Architecture Decision Records

Las decisiones arquitectónicas importantes deberán documentarse mediante ADR.

Ejemplos:

```text
ADR-001 Elección de PostgreSQL
ADR-002 Uso de Rust para autenticación
ADR-003 Uso de Elixir/Phoenix como Core
ADR-004 Uso de Cassandra para mensajería e historial
ADR-005 Uso de JavaFX para escritorio
ADR-006 Uso de Vagrant y Docker Compose para el entorno de desarrollo
```

Formato:

```text
Título
Estado
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

**Versión actual:** 0.2  
**Próxima revisión:** después del diseño de arquitectura y del primer módulo de autenticación.

Este estándar se considera un documento vivo.