# Sonimbus

Plataforma distribuida de comunicación y gestión ligera de proyectos para grupos académicos, equipos de trabajo y comunidades pequeñas: grupos, canales de texto y voz, mensajería en tiempo real, archivos, proyectos y tareas, notificaciones internas y reportes básicos de actividad.

Dos clientes ricos (escritorio y móvil) consumen la misma API a través de un gateway. El backend está formado por servicios independientes que corren en contenedores sobre GNU/Linux.

Proyecto de las materias **Desarrollo de Sistemas en Red** (NRC 20226) y **Despliegue de Software** (NRC 20222), Facultad de Estadística e Informática, Universidad Veracruzana.

## Equipo y módulos

| Módulo | Ruta | Stack | Responsable |
|---|---|---|---|
| Cliente de escritorio | [`clients/desktop/`](clients/desktop/) | Java 21, JavaFX 21, Maven, SQLite, LiveKit (WebRTC) | [@Maple-M136279841](https://github.com/Maple-M136279841) |
| Cliente móvil | [`clients/mobile/`](clients/mobile/) | Flutter, Dart, Drift (SQLite), LiveKit | [@roluva](https://github.com/roluva) |
| Auth Service | [`services/auth/`](services/auth/) | Rust, Axum, Tokio, SQLx | [@tuzc0](https://github.com/tuzc0) |
| Core Service | [`services/core/`](services/core/) | Elixir, Phoenix, OTP | [@Maple-M136279841](https://github.com/Maple-M136279841) |
| File Service | [`services/files/`](services/files/) | Elixir, MinIO | [@Maple-M136279841](https://github.com/Maple-M136279841) |
| Report Service | [`services/reports/`](services/reports/) | Por decidir (propuesta: Elixir) | [@roluva](https://github.com/roluva) |
| API Gateway | [`gateway/nginx/`](gateway/nginx/) | Nginx | [@tuzc0](https://github.com/tuzc0) |
| Infraestructura | [`infrastructure/`](infrastructure/) | Vagrant, Docker Compose, LiveKit | [@tuzc0](https://github.com/tuzc0) |

El reparto completo, las fases y las tareas abiertas están en [`docs/PLAN.md`](docs/PLAN.md).

## Arquitectura

| Capa | Componentes |
|---|---|
| Clientes | Escritorio (JavaFX) y móvil (Flutter), con almacenamiento local en SQLite |
| Entrada | Nginx: TLS, REST, WebSocket y límite de peticiones |
| Servicios | Auth (identidad, tokens y perfil), Core (grupos, mensajería, proyectos, notificaciones y control de voz), Files (archivos) y Reports (reportes) |
| Comunicación | REST, WebSocket (canales de Phoenix), gRPC entre servicios (`checkAccess`), RabbitMQ para eventos y WebRTC con LiveKit para la voz |
| Persistencia | PostgreSQL (un esquema por servicio), ScyllaDB (mensajes), Redis (sesiones, presencia y caché) y MinIO (archivos) |

Detalle en [`docs/architecture/`](docs/architecture/) y decisiones en [`docs/decisions/`](docs/decisions/).

## Estructura del repositorio

```text
.
├── .github/          plantillas de issues y PR, CODEOWNERS
├── clients/          desktop/ (JavaFX) y mobile/ (Flutter)
├── services/         auth/ (Rust), core/, files/ y reports/
├── gateway/nginx/    API gateway
├── contracts/        OpenAPI, eventos y gRPC: el acuerdo entre módulos
├── database/         inicialización de PostgreSQL y esquemas CQL de ScyllaDB
├── infrastructure/   Docker Compose, .env.example y LiveKit
├── scripts/          aprovisionamiento de la VM
├── docs/             requisitos, arquitectura, decisiones, plan y estándares
└── Vagrantfile
```

## Entorno de desarrollo

El entorno local se ejecuta sobre una máquina virtual común para todo el equipo (D-14, [ADR-006](docs/decisions/ADR-006-vagrant-docker-compose.md)):

- Ubuntu Server 24.04 LTS;
- Vagrant y VirtualBox;
- Docker Engine;
- Docker Compose.

La máquina virtual utiliza:

- 6 GB de RAM;
- 4 CPU virtuales;
- disco principal de 80 GB;
- IP privada `192.168.33.30`.

### Servicios de desarrollo

| Servicio | Inicio | Puerto | Propósito |
|---|---|---:|---|
| PostgreSQL | Predeterminado | 5432 | Persistencia relacional |
| Redis | Predeterminado | 6379 | Caché e información temporal |
| Mailpit | Predeterminado | 1025 / 8025 | Pruebas locales de correo |
| Cassandra | Perfil `messaging` | 9042 | Mensajería e historial (se reemplaza por ScyllaDB, T-01) |
| MinIO | Perfil `storage` | 9000 / 9001 | Almacenamiento de objetos |

## Inicio rápido

Requisitos del equipo anfitrión:

- Vagrant 2.4.9 o una versión compatible;
- VirtualBox 7.2 o una versión compatible;
- virtualización de hardware habilitada;
- al menos 16 GB de RAM;
- conexión a Internet durante el primer aprovisionamiento.

Crear la configuración local en PowerShell:

```powershell
Copy-Item .\infrastructure\.env.example .\infrastructure\.env
```

El archivo `infrastructure/.env` no debe subirse al repositorio. Antes de iniciar la máquina, deberán sustituirse sus valores de ejemplo por credenciales locales.

Crear y aprovisionar la VM:

```powershell
vagrant validate
vagrant up --provider=virtualbox
vagrant ssh
```

PostgreSQL, Redis y Mailpit se inician durante el aprovisionamiento. Para revisar su estado dentro de la VM:

```bash
cd /vagrant/infrastructure
docker compose ps
```

Iniciar la base de mensajes (hoy Cassandra; ScyllaDB tras T-01):

```bash
docker compose --profile messaging up -d
```

Iniciar MinIO:

```bash
docker compose --profile storage up -d
```

Iniciar todos los servicios:

```bash
docker compose --profile messaging --profile storage up -d
```

Interfaces disponibles desde el equipo anfitrión:

- Mailpit: `http://192.168.33.30:8025`
- MinIO Console: `http://192.168.33.30:9001`

Detener los contenedores sin eliminar los datos:

```bash
docker compose --profile messaging --profile storage down
```

> No utilice `docker compose down -v` salvo que se desee eliminar permanentemente los datos locales.

## Seguridad

- Los archivos `.env` reales no deben versionarse.
- `.env.example` únicamente debe contener valores de referencia.
- No deben incorporarse contraseñas, tokens, claves privadas ni certificados al repositorio.
- Los datos persistentes son administrados mediante volúmenes nombrados de Docker dentro de la VM.

## Documentación

| Documento | Para qué sirve |
|---|---|
| [Plan de trabajo](docs/PLAN.md) | Equipo, reparto, fases y tareas abiertas |
| [Requisitos](docs/requirements/) | Casos de uso, requisitos y trazabilidad por módulo |
| [Arquitectura](docs/architecture/) | Componentes, comunicación, datos y concurrencia |
| [Decisiones](docs/decisions/) | Registro de decisiones técnicas y ADR |
| [Modelo de datos](docs/database/) | Esquemas de PostgreSQL y diseño de ScyllaDB |
| [Despliegue](docs/deployment/) | Servidor de la expo y checklist |
| [Estándar de desarrollo](docs/STANDARD_DEVELOPMENT.md) | Normas técnicas del proyecto |
| [Definition of Done](docs/DEFINITION_OF_DONE.md) | Criterios para cerrar una tarea |
| [Guía de contribución](CONTRIBUTING.md) | Ramas, commits, PR y revisión |

## Estado

🚧 Inicio de la codificación: infraestructura base y esqueleto de Auth listos. Ver [`docs/PLAN.md`](docs/PLAN.md).
