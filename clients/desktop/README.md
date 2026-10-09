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

**Inicializado (Esqueleto de UI completado).** Todas las ventanas y vistas del MVP han sido creadas. Actualmente, los controladores son esqueletos (los botones aún no ejecutan acciones de negocio), listos para comenzar la integración con la lógica y el gateway.

### Navegación de demostración (Temporal)

Para facilitar la revisión y exposición de las interfaces, se incluyó una lógica temporal en `App.java` que permite saltar directamente a cualquier vista o iterar sobre ellas usando argumentos de ejecución en JavaFX. 

Para navegar por las vistas secuencialmente, ejecuta el siguiente comando o configura tu IDE (Run → Edit Configurations) con los siguientes parámetros:

```bash
mvn -pl app javafx:run -Djavafx.args=siguiente
