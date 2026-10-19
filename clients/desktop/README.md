# Cliente de escritorio

**Responsable principal:** `@Maple-M136279841`

## Responsabilidad

Aplicación de escritorio de Sonimbus: interfaz de usuario, almacenamiento local y comunicación con el backend a través del gateway (REST y WebSocket). Incluye la comunicación por voz con LiveKit (D-01).

Cubre todos los casos de uso del MVP y concentra la administración detallada (RES-06). Ver la trazabilidad en [`docs/requirements/`](../../docs/requirements/README.md#cliente-de-escritorio).

## Stack

- Java 21 y JavaFX 21.
- Maven, como proyecto multimódulo.
- SQLite para almacenamiento local cuando corresponda.
- `webrtc-java` 0.18.0 para audio WebRTC.

## Módulos

| Módulo | Contenido |
|---|---|
| [`app/`](app/) | El cliente JavaFX. |
| [`livekit-sdk/`](livekit-sdk/) | Fork adoptado del SDK comunitario de LiveKit para Java. |

`app/` usa la voz únicamente a través de una interfaz propia en su capa de negocio; ninguna otra parte del cliente depende directamente de `livekit-sdk/`. Así puede sustituirse la implementación de voz sin afectar al resto de la aplicación.

## Fuera de alcance

- Lógica de negocio que pertenece a los servicios (permisos, validaciones de dominio).
- Generación de tokens de LiveKit: el cliente los recibe de Core (D-15); nunca contiene la clave del servidor.

## Contratos que consume

- [`contracts/openapi/`](../../contracts/openapi/)
- [`contracts/events/`](../../contracts/events/)

## Referencias

- Prueba de concepto de voz: https://github.com/sonimbus-uv/poc-voice-javafx

## Estado

**Conectado a Auth Service en lo que Auth ofrece hoy (Fase 5):** registro, verificación de correo por enlace y reenvío. Las 16 vistas del MVP existen; las que dependen de servicios que aún no existen son solo interfaz.

| Área | Estado |
|---|---|
| Capa de API de Auth (`data/api`) | **Lista.** Registro, reenvío de verificación y verificación por token, con errores tipados y pruebas contra un servidor HTTP local. |
| Configuración de la URL de Auth | **Lista.** Ver [Ejecución](#ejecución). |
| Navegación (vistas, shell, diálogos, parámetros, inyección de dependencias) | **Lista.** |
| Pantalla de registro (CU-01) | **Conectada.** Validación local idéntica a la de Auth y mensajes en español para 400, 409, 429, red, tiempo agotado y 5xx. El campo «Nombre completo» está oculto hasta resolver #14. |
| Pantalla de verificación (CU-02) | **Conectada.** Adaptada al enlace del correo, con reenvío y espera de 60 s. |
| Inicio de sesión, recuperación de contraseña y Google (CU-03, CU-05, CU-04) | **No disponibles:** Auth aún no tiene esos endpoints. Las pantallas validan los campos y avisan que la función no está disponible. |
| Grupos, chat, voz, archivos, proyectos, reportes, notificaciones, perfil | Solo interfaz. Core, Files y Reports aún no existen. |

Las decisiones de esta fase están en [`docs/fase-auth-notas.md`](docs/fase-auth-notas.md).

## Requisitos

- **Java 21.** El build falla con otra versión, porque `maven-enforcer-plugin` la exige. Si el sistema trae otra por defecto:

  ```bash
  export JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64
  ```

- Maven 3.8.7 o posterior.

Todos los comandos se ejecutan desde `clients/desktop/`.

## Ejecución

```bash
mvn -pl app javafx:run
```

Para ejecutarlo desde el IDE, usa como clase principal `mx.uv.sonimbus.desktop.Launcher`.

### URL de Auth Service

El cliente toma la URL de Auth de la primera de estas fuentes que esté definida:

1. Propiedad de sistema `sonimbus.auth.url`.
2. Variable de entorno `SONIMBUS_AUTH_URL`.
3. Valor por defecto: `http://192.168.33.30:3001`, que es Auth en la VM de Vagrant.

```bash
SONIMBUS_AUTH_URL=http://localhost:3001 mvn -pl app javafx:run
```

En el IDE, agrega `-Dsonimbus.auth.url=http://localhost:3001` en **VM options**. Con Maven usa la variable de entorno: `javafx:run` lanza la aplicación en otro proceso, y una propiedad `-D` pasada a Maven no le llega.

La URL debe ser `http` o `https`, con host y **sin credenciales**; si no lo es, la aplicación no arranca. Los tiempos de espera son de 5 s para conectar y 10 s por petición (RNF-REN-01).

### Probar el registro de punta a punta con la VM

1. Levanta la VM desde la raíz del repositorio con `vagrant up --provider=virtualbox` o `--provider=libvirt` y entra con `vagrant ssh` (requisitos de cada proveedor en el [README raíz](../../README.md#inicio-rápido)). El aprovisionamiento inicia PostgreSQL, Redis y Mailpit; compruébalo con:

   ```bash
   cd /vagrant/infrastructure && docker compose ps
   ```

2. Comprueba que la VM tenga Rust (`cargo --version`). Si no lo tiene, instálalo como usuario `vagrant`:

   ```bash
   sudo apt-get install -y build-essential pkg-config
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --profile minimal
   . "$HOME/.cargo/env"
   ```

3. Crea `services/auth/.env` a partir de `.env.example` (`cp .env.example .env`). Ya trae Mailpit configurado:

   ```env
   SERVER_ADDR=0.0.0.0:3001
   EMAIL_PROVIDER=mailpit
   MAILPIT_SMTP_HOST=localhost
   MAILPIT_SMTP_PORT=1025
   EMAIL_VERIFICATION_URL=http://localhost:3001/auth/verify-email
   ```

   El enlace debe quedarse en `localhost`: Auth solo acepta `http` en `EMAIL_VERIFICATION_URL` para `localhost` o `127.0.0.1`, y con la IP de la VM no arranca.

   **Ajusta `DATABASE_URL`**: debe usar el usuario, la contraseña y la base de `infrastructure/.env` (`POSTGRES_USER`, `POSTGRES_PASSWORD`, `POSTGRES_DB`), que es lo que levanta Docker. Los dos `.env.example` todavía no coinciden (T-02 en [`docs/PLAN.md`](../../docs/PLAN.md)); si no se ajusta, Auth falla con «password authentication failed». Si la contraseña tiene caracteres especiales, deben ir codificados para URL.

   ```env
   DATABASE_URL=postgres://<POSTGRES_USER>:<POSTGRES_PASSWORD>@localhost:5432/<POSTGRES_DB>
   ```

4. Inicia Auth dentro de la VM (`cargo run` en `/vagrant/services/auth/`) y comprueba desde tu equipo que responde: `curl http://192.168.33.30:3001/health`.
5. En tu equipo, abre un túnel para que el `localhost:3001` del enlace llegue a la VM. Déjalo abierto en otra terminal:

   ```bash
   vagrant ssh -- -N -L 3001:localhost:3001
   ```

6. Ejecuta el cliente con la URL por defecto: `mvn -pl app javafx:run`.
7. Regístrate. El correo de verificación aparece en Mailpit, en **http://192.168.33.30:8025**. Al abrir el enlace, que gracias al túnel llega a Auth en la VM, la cuenta queda verificada.
8. En la pantalla de verificación, «Reenviar correo» se habilita a los 60 s y manda un nuevo enlace a Mailpit. «Ya verifiqué mi cuenta» vuelve al inicio de sesión, que por ahora solo avisa que no está disponible.

## Navegación de demostración

**Temporal**, mientras no exista el inicio de sesión: el primer argumento de la aplicación abre directamente cualquier vista, para revisarlas y exponerlas.

```bash
mvn -pl app javafx:run -Djavafx.args=siguiente        # una vista por ejecución, en orden
mvn -pl app javafx:run -Djavafx.args=8                # por número
mvn -pl app javafx:run -Djavafx.args=verify-account   # por nombre (sin distinguir mayúsculas)
```

En el IDE (Run → Edit Configurations), pon `siguiente`, el número o el nombre en **Program arguments**.

- `siguiente` recuerda la última vista mostrada (en las preferencias del usuario) y al llegar a la 16 vuelve a la 1.
- Las vistas de auth y el shell ocupan la ventana. Las secciones se muestran dentro del shell con su título, y los diálogos se abren como ventana modal sobre el shell.
- El título de la ventana indica la vista: «Sonimbus · 3 de 16 · VERIFY_ACCOUNT».
- Si el argumento no se reconoce, se imprime la lista de vistas y se abre el inicio de sesión.

| # | Vista | Se muestra |
|---|---|---|
| 1 | `LOGIN` | Ventana completa |
| 2 | `REGISTER` | Ventana completa |
| 3 | `VERIFY_ACCOUNT` | Ventana completa |
| 4 | `RECOVER_PASSWORD` | Ventana completa |
| 5 | `MAIN` | Ventana completa (shell) |
| 6 | `TEXT_CHANNEL` | Dentro del shell |
| 7 | `VOICE_CHANNEL` | Dentro del shell |
| 8 | `SHARED_FILES` | Dentro del shell |
| 9 | `CREATE_GROUP` | Diálogo |
| 10 | `JOIN_GROUP` | Diálogo |
| 11 | `GROUP_SETTINGS` | Dentro del shell |
| 12 | `PROJECTS` | Dentro del shell |
| 13 | `TASK_DETAIL` | Diálogo |
| 14 | `REPORTS` | Dentro del shell |
| 15 | `NOTIFICATIONS` | Dentro del shell |
| 16 | `PROFILE` | Dentro del shell |

## Pruebas

```bash
mvn verify                          # compila y ejecuta todas las pruebas
mvn -pl app test -Dtest=AuthApiTest # una clase
python3 docs/check_fxml.py          # revisión estática de los FXML (debe dar 0 errores)
```

Las pruebas no necesitan pantalla ni red:

- **Capa de API:** contra un servidor HTTP local (`com.sun.net.httpserver.HttpServer`) que simula las respuestas de Auth: 201, 202, 400, 409, 429 con `Retry-After`, 500, 503, cuerpo malformado, servidor caído y tiempo agotado. También comprueban que la contraseña y el token no aparezcan en mensajes ni trazas.
- **Validación del registro:** casos límite de la política de Auth (7/8 y 128/129 caracteres, requisitos, Unicode y correos).
- **Pantallas de autenticación:** TestFX sobre JavaFX real, sin pantalla gracias a Monocle (configurado en `app/pom.xml`), con `AuthApi` simulado con Mockito.

Para ver las pruebas de interfaz en una ventana real, quita las propiedades `glass.platform`/`monocle.platform`/`testfx.headless` del surefire de `app/pom.xml`.

## Estructura de paquetes

Paquete base `mx.uv.sonimbus.desktop` (`app/src/main/java/`):

| Paquete | Contenido |
|---|---|
| *(raíz)* | `App` (arranque), `Launcher` (entrada para IDE/JAR), `AppContext` (crea los servicios y los inyecta) y `DemoNavigation` (temporal). |
| `data/api` | Clientes HTTP de los servicios (`AuthApi`), configuración (`ApiConfig`, `AuthPaths`) y errores tipados (`ApiException`, `ApiErrorKind`). No dependen de JavaFX. |
| `data/local` | Almacenamiento local (SQLite). Vacío por ahora. |
| `data/realtime` | WebSocket con el gateway. Vacío por ahora. |
| `domain/session` | Sesión del usuario y reglas del cliente sin JavaFX, como la validación del registro. |
| `domain/outbox` | Cola local de operaciones pendientes (RNF-CON-01). Vacío por ahora. |
| `domain/voice` | Interfaz propia de voz; la implementación usará `livekit-sdk/`. Vacío por ahora. |
| `ui/navigation` | `View` (vistas y sus FXML), `ViewManager` (ventana completa, shell, diálogos, parámetros), `ControllerFactory`, `Navigable`, `Parameterized` y `ContentHost`. |
| `ui/common` | Utilidades de interfaz compartidas, como `BackgroundTasks`, que ejecuta llamadas fuera del hilo de JavaFX. |
| `ui/<área>` | Controladores de cada área (`auth`, `shell`, `chat`, `voice`, `files`, `groups`, `projects`, `reports`, `notifications`, `profile`). |

Recursos en `app/src/main/resources/mx/uv/sonimbus/desktop/`: los FXML en `ui/<área>/`, el tema en `css/theme.css` (variables `-sn-*`) e imágenes en `images/`. Las convenciones de las vistas están en [`docs/vistas-javafx.md`](docs/vistas-javafx.md).

### Cómo recibe dependencias un controlador

Declara un único constructor público con lo que necesita; `ControllerFactory` se lo entrega:

```java
public RegisterController(AuthApi authApi, BackgroundTasks tasks, ViewManager viewManager) { ... }
```

Para recibir datos de la vista anterior, implementa `Parameterized<P>`:

```java
viewManager.show(View.VERIFY_ACCOUNT, VerifyAccountParams.afterRegistration(email, emailSent));
```
