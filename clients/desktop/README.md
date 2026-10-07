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

Sin inicializar. El primer PR del responsable crea el proyecto (`pom.xml` padre y los módulos `app` y `livekit-sdk`) con las versiones exactas de su entorno y actualiza este README con los pasos para compilar y ejecutar.
