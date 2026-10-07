# Registro de decisiones

Decisiones técnicas vigentes de Sonimbus. Cada una tiene un identificador `D-NN` que puede citarse en issues, PR y documentos.

Una decisión que cambia la arquitectura, que tiene alternativas serias o que es costosa de revertir se desarrolla además como ADR en esta carpeta, con el formato de la sección 31 de [`STANDARD_DEVELOPMENT.md`](../STANDARD_DEVELOPMENT.md). Este registro es el índice: si una decisión y un ADR no coinciden, se corrige el que esté desactualizado en el mismo PR.

**Fuente:** documento de traspaso a codificación (etapa de especificación) y acuerdos del equipo posteriores.

## Decisiones vigentes

| ID | Decisión | Estado | Fecha | ADR |
|---|---|---|---|---|
| D-01 | La voz entra al MVP | Aceptada | 2026-10 | Pendiente |
| D-02 | Nginx como API Gateway; cada servicio valida el JWT | Aceptada | 2026-10 | Pendiente |
| D-03 | File Service en Elixir | Aceptada | 2026-10 | — |
| D-04 | RabbitMQ dentro del MVP | Aceptada | 2026-10 | Pendiente |
| D-05 | **ScyllaDB** para los mensajes | Aceptada (modificada el 2026-10-07) | 2026-10-07 | Pendiente |
| D-06 | El perfil de usuario vive en Auth Service | Aceptada | 2026-10 | — |
| D-07 | Tokens JWT firmados con EdDSA (Ed25519) | Aceptada | 2026-10 | — |
| D-08 | El orden de los mensajes lo asigna el servidor | Aceptada | 2026-10 | — |
| D-09 | Infraestructura en Docker Compose con volúmenes con nombre | Aceptada | 2026-09-17 | [ADR-006](ADR-006-vagrant-docker-compose.md) |
| D-10 | La laptop de Guillermo es el servidor de la expo | Aceptada | 2026-10 | — |
| D-11 | Contraseñas con Argon2id | Aceptada | 2026-10 | — |
| D-12 | Una sesión activa por tipo de cliente | Aceptada | 2026-10 | — |
| D-13 | PostgreSQL: una base de datos con un esquema y un usuario por servicio | Aceptada | 2026-10-07 | — |
| D-14 | Desarrollo en VM Vagrant; expo en Ubuntu nativo | Aceptada | 2026-10-07 | [ADR-006](ADR-006-vagrant-docker-compose.md) |
| D-15 | VoiceService (en Core) emite los tokens de LiveKit | Aceptada | 2026-10 | — |

## Detalle

### D-01. La voz entra al MVP

La prueba de concepto de JavaFX con LiveKit autoalojado fue exitosa (riesgo R-01). La AR02 marcaba la voz como "condicionada"; esa condición queda resuelta y CU-22 pasa a ser parte del MVP.

- Escritorio: `webrtc-java` 0.18.0 y un fork del SDK comunitario `Trirrin/livekit-java-sdk` v0.1.4, aislado detrás de una interfaz propia (ver [`clients/desktop/livekit-sdk/`](../../clients/desktop/livekit-sdk/)).
- Móvil: SDK oficial de LiveKit para Flutter.
- Riesgo abierto: el SDK comunitario no es oficial y tiene defectos conocidos; el defecto 4 del fork debe resolverse antes de integrar la voz en la aplicación.

### D-02. Nginx como API Gateway

Acordado con el profesor. La versión libre de Nginx no valida JWT, así que **cada servicio valida el token** con la clave pública que publica Auth Service. El gateway se encarga de TLS, enrutamiento REST, `upgrade` de WebSocket, `grpc_pass` (si se usa gRPC) y `limit_req`.

| Gateway | A favor | En contra |
|---|---|---|
| Nginx (elegido) | Maduro, conocido, WebSocket, `grpc_pass`, `limit_req` | Sin validación de JWT en la versión libre |
| Traefik | Descubrimiento automático en Docker, TLS automático | JWT solo con plugins o *forwardAuth* |
| Envoy | El mejor soporte de gRPC y JWT nativo | Configuración compleja |
| Kong | Plugins de JWT y límite de peticiones | Un componente más que operar |

### D-03. File Service en Elixir

Reduce el número de lenguajes del backend. Rust queda solo en Auth Service.

### D-04. RabbitMQ dentro del MVP

Difiere de RNF-INT-02 de la AR02, que lo dejaba opcional. Topología:

- Un *exchange* `topic` llamado `sonimbus.events`.
- Claves de ruteo con el formato `<recurso>.<evento>`: `message.created`, `task.updated`, `user.updated`, `member.joined`, etc.
- Una cola durable por consumidor y una cola de mensajes fallidos (*dead-letter queue*).
- *Publisher confirms* en los productores y *ack* manual en los consumidores.
- Bibliotecas: `Broadway` + `broadway_rabbitmq` para consumir y `AMQP` para publicar (Elixir); `lapin` (Rust).

El catálogo de eventos vive en [`contracts/events/`](../../contracts/events/).

### D-05. ScyllaDB para los mensajes

**Cambio del 2026-10-07.** El traspaso elegía Cassandra con ScyllaDB como alternativa; el equipo eligió ScyllaDB para el MVP.

- Motivo: el entorno tiene recursos limitados (RES-07) y la máquina de desarrollo tiene 6 GB. ScyllaDB corre con un núcleo y una memoria acotada (`--smp 1 --memory <N> --overprovisioned 1`), en lugar del *heap* de 1 GB o más de Cassandra.
- Compatibilidad: ScyllaDB usa el mismo CQL y funciona con el mismo driver (Xandra en Elixir), así que el diseño de tablas no cambia.
- Pendiente: `infrastructure/compose.yaml` todavía levanta `cassandra:5.0.9`; el cambio está registrado como tarea T-01 en [`docs/PLAN.md`](../PLAN.md).

El diseño de las tablas está en [`docs/database/`](../database/).

### D-06. El perfil de usuario vive en Auth Service

Auth es dueño del nombre visible y de la foto de perfil. Los demás servicios mantienen una copia local de nombre y foto, actualizada con el evento `user.updated`.

La foto se elige de un **paquete predefinido** incluido en los clientes (CU-07): Auth solo guarda el identificador de la imagen y no depende del File Service.

### D-07. Tokens JWT firmados con EdDSA (Ed25519)

Auth firma; los demás servicios verifican con la clave pública. Hay que verificar el soporte de Ed25519 en `jsonwebtoken` (Rust) y en `Joken`/`JOSE` (Elixir) antes de cerrar el contrato.

### D-08. El orden de los mensajes lo asigna el servidor

Timeuuid por canal al almacenar. Las horas se guardan en UTC y cada cliente las muestra en su zona horaria local. El reloj del cliente nunca decide el orden.

### D-09. Infraestructura en Docker Compose

Todo el servidor, incluidas las bases de datos, corre en contenedores con volúmenes con nombre de Docker (no carpetas montadas). Ver [ADR-006](ADR-006-vagrant-docker-compose.md).

### D-10. La laptop de Guillermo es el servidor de la expo

ASUS TUF, 16 GB de RAM, Ubuntu nativo. La migración posterior a un VPS consiste en copiar el `compose`, el `.env` y los volúmenes. Ver [`docs/deployment/`](../deployment/).

### D-11. Contraseñas con Argon2id

Cumple RNF-SEG-01.

### D-12. Una sesión activa por tipo de cliente

RES-04: una en escritorio y una en móvil. Implementado en la migración inicial de Auth (`auth.open_session`: el último inicio de sesión gana).

### D-13. PostgreSQL: un esquema y un usuario por servicio

**Acordado el 2026-10-07.** En el MVP todos los servicios comparten una instancia y una base de datos de PostgreSQL. Cada servicio tiene su propio esquema (`auth`, `core`, `files`, `reports`) y su propio usuario, con permisos solo sobre su esquema. Ningún servicio lee el esquema de otro.

Coincide con la migración actual de Auth, que crea el esquema `auth`.

### D-14. Desarrollo en VM Vagrant; expo en Ubuntu nativo

- **Desarrollo:** cada integrante usa la VM de Vagrant + VirtualBox (Ubuntu Server 24.04, 6 GB) descrita en [ADR-006](ADR-006-vagrant-docker-compose.md). Es lo que ya existe en el repositorio.
- **Expo:** el servidor corre en Ubuntu nativo (D-10) con el mismo `compose.yaml`.
- Quien trabaje en Ubuntu nativo puede usar Docker directamente, pero el repositorio debe estar en ext4 (no NTFS) y Java 21 debe ser el JDK predeterminado.

### D-15. VoiceService emite los tokens de LiveKit

El plano de control de la voz vive en Core (VoiceService). Core verifica los permisos del miembro sobre el canal y genera el JWT de LiveKit con `Joken`, usando la clave y el secreto de la API de LiveKit. Los clientes nunca contienen el secreto.

## Decisiones pendientes

Se resuelven mediante issue o PoC y se agregan a la tabla anterior al cerrarse.

| Tema | Opciones o valores por definir | Responsable |
|---|---|---|
| Transferencia de archivos | gRPC con streaming frente a REST con subida por partes y `Range`. PoC: 50 MB por bloques, de JavaFX a Elixir pasando por Nginx | @Maple-M136279841 |
| Lenguaje de Report Service | Propuesta: Elixir | @roluva |
| Tokens | Vigencia del token de acceso y del de renovación; rotación del de renovación en cada uso; almacenamiento seguro en JavaFX y Flutter | @tuzc0 |
| Google OAuth en escritorio | Propuesta: redirección a *loopback* con PKCE; credenciales en Google Cloud Console | @tuzc0 |
| Catálogo de eventos de RabbitMQ | Nombre, productor, consumidores y contenido de cada evento | Cada productor |
| Valores concretos | Longitud máxima de mensaje; tamaño y tipos de archivo; caducidad de códigos; política de contraseñas; intentos fallidos antes de bloquear; tamaño de página del historial; rango máximo de reportes; valores por defecto de invitaciones | Dueño de cada servicio |
