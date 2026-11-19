# Cliente de escritorio

**Responsable principal:** `@Maple-M136279841`

## Responsabilidad

Aplicación de escritorio de Sonimbus: interfaz de usuario, almacenamiento local y comunicación con el backend a través del gateway (REST y WebSocket). Incluye la comunicación por voz con LiveKit (D-01).

Su alcance previsto cubre los casos de uso del MVP y la administración detallada
(RES-06). La trazabilidad se encuentra en
[`docs/requirements/`](../../docs/requirements/README.md#cliente-de-escritorio).

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

El módulo de Auth integra registro, verificación de correo, login con contraseña,
renovación de sesión, logout, recuperación y cambio de contraseña, inicio de sesión
con Google y vinculación explícita desde el perfil.

La referencia de arquitectura y mantenimiento es [Auth de escritorio](docs/registro-actual.md).

| Área | Estado |
|---|---|
| Auth | Integrado con servicios de dominio y adaptadores HTTP por funcionalidad. |
| Navegación | Vistas, shell, diálogos, parámetros y control de acceso por sesión. |
| Apariencia | Claro y Oscuro con cambio inmediato y preferencia local. |
| Perfil | Cambio de contraseña, logout y vinculación de Google integrados. |
| Otras áreas | Conservan sus vistas; su presencia no implica conexión con sus servicios. |

## Requisitos

- **Java 21.** El build falla con otra versión, porque `maven-enforcer-plugin` la exige. La selección de Java 21 puede realizarse con JAVA_HOME:

  ```bash
  export JAVA_HOME=/usr/lib/jvm/java-21-openjdk-amd64
  ```

- Maven 3.8.7 o posterior.

Los comandos Maven se ejecutan desde `clients/desktop/`. Los scripts de desarrollo
indican su directorio de ejecución en la sección correspondiente.

## Ejecución

```bash
mvn -pl app javafx:run
```

La clase principal para la ejecución desde el IDE es `mx.uv.sonimbus.desktop.Launcher`.

### URL de Auth Service y de Core Service

El cliente toma la URL de cada servicio de la primera de estas fuentes que esté definida:

| Fuente | Auth | Core |
|---|---|---|
| Propiedad de sistema | `sonimbus.auth.url` | `sonimbus.core.url` |
| Variable de entorno | `SONIMBUS_AUTH_URL` | `SONIMBUS_CORE_URL` |
| Valor por defecto | `http://192.168.33.30:3001` (Auth en la VM) | `http://localhost:3002` (Core por un túnel SSH) |

El WebSocket de mensajes usa el host de Core (`ws://…/socket/websocket`). Para la demo con Core, el túnel y el orden de arranque están en [`docs/deployment/demo-expo-escritorio.md`](../../docs/deployment/demo-expo-escritorio.md).

```bash
SONIMBUS_AUTH_URL=http://localhost:3001 mvn -pl app javafx:run
```

En el IDE se configura `-Dsonimbus.auth.url=http://localhost:3001` en **VM options**. Con Maven se utiliza la variable de entorno: `javafx:run` lanza la aplicación en otro proceso, y una propiedad `-D` pasada a Maven no le llega.

La URL debe ser `http` o `https`, con host y **sin credenciales**; si no lo es, la aplicación no arranca. Los tiempos de espera son de 5 s para conectar y 10 s por petición (RNF-REN-01).

### Entorno de desarrollo de Auth

El script existente inicia las dependencias, el gateway y Auth dentro de la VM.
Desde `/vagrant`, el modo de desarrollo con login habilitado se inicia mediante:

```bash
bash scripts/auth/dev/start-auth-dev.sh --enable-login
```

En Windows, los scripts existentes preparan la confianza de la CA en el sistema y
el túnel al gateway. Desde la raíz del repositorio:

```powershell
powershell -ExecutionPolicy Bypass -File scripts/auth/dev/trust-gateway-dev-ca.ps1
powershell -ExecutionPolicy Bypass -File scripts/auth/dev/connect-auth-dev.ps1
```

El túnel permanece abierto durante la ejecución. El cliente utiliza
`SONIMBUS_AUTH_URL=https://localhost:8443` para acceder por ese gateway.
La confianza del sistema para el navegador no sustituye la configuración de
certificados de la JVM, que también debe confiar en la CA.
Mailpit queda accesible por el túnel en `http://localhost:18025`.
La verificación y la recuperación utilizan códigos de ocho dígitos, no enlaces
de activación. La configuración y los secretos del proveedor Google permanecen
en el backend.

## Navegación de demostración

El modo de demostración abre vistas mediante el primer argumento de la aplicación.
Omite deliberadamente la comprobación de sesión para la inspección visual; no
representa una sesión autenticada ni concede autorización en el backend.

```bash
mvn -pl app javafx:run -Djavafx.args=siguiente        # una vista por ejecución, en orden
mvn -pl app javafx:run -Djavafx.args=8                # por número
mvn -pl app javafx:run -Djavafx.args=verify-account   # por nombre (sin distinguir mayúsculas)
```

En el IDE, **Program arguments** admite `siguiente`, el número o el nombre de la vista.

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

Las pruebas no necesitan pantalla ni conexión con Auth o Google reales. Las pruebas
HTTP y de comunicación entre procesos utilizan la red local de loopback:

- **Capa de API:** contra un servidor HTTP local (`com.sun.net.httpserver.HttpServer`) que simula las respuestas de Auth: 201, 202, 400, 409, 429 con `Retry-After`, 500, 503, cuerpo malformado, servidor caído y tiempo agotado. También comprueban que la contraseña y el token no aparezcan en mensajes ni trazas.
- **Validación del registro:** casos límite de la política de Auth (7/8 y 128/129 caracteres, requisitos, Unicode y correos).
- **Pantallas de autenticación:** TestFX sobre JavaFX real, sin pantalla gracias a Monocle (configurado en `app/pom.xml`), con `AuthApi` simulado con Mockito.

La configuración de TestFX y Monocle se encuentra en Surefire, en `app/pom.xml`.
La ejecución habitual no necesita una ventana visible.

## Estructura de paquetes

El paquete base es `mx.uv.sonimbus.desktop`, dentro de `app/src/main/java/`.

| Paquete | Responsabilidad |
|---|---|
| Raíz | Arranque, composición en `AppContext` y navegación de demostración. |
| `domain/{registration,session,password,google}` | Casos de uso, modelos, errores y gateways de Auth. |
| `data/api/auth/{registration,session,password,google}` | Adaptadores HTTP y DTO por funcionalidad. |
| `data/browser/google`, `data/instance`, `data/local` | Retorno OAuth, instancia única y archivos privados. |
| `data/logging` | Registro seguro de acontecimientos de autenticación. |
| `ui/auth/{common,registration,login,recovery}`, `ui/profile` | Formularios y gestión de credenciales. |
| `ui/navigation`, `ui/common`, `ui/theme` | Navegación, tareas en segundo plano y apariencia. |
| Otras áreas | Vistas y puntos de extensión de las demás funcionalidades. |

Los recursos FXML conservan la organización de sus controladores.
`css/theme.css` define los colores por función mediante valores consultables `-sn-*`.
Las referencias de interfaz son [vistas JavaFX](docs/vistas-javafx.md) y
[temas y paleta](docs/temas-y-paleta.md).

`ControllerFactory` entrega por constructor las dependencias registradas en
`AppContext`. Los controladores reciben servicios de dominio, `BackgroundTasks`
y `ViewManager` según su responsabilidad. Los datos entre pantallas se transmiten
mediante `Parameterized<P>` y modelos como `VerifyAccountParams`.
