# Cliente móvil

**Responsable principal:** `@tuzc0`  
**Apoyo:** `@Maple-M136279841`

## Responsabilidad

Aplicación móvil de Sonimbus: interfaz de usuario, funcionamiento sin conexión y comunicación con el backend a través del gateway (REST y WebSocket).

## Stack

- Flutter y Dart.
- SQLite mediante Drift para almacenamiento local y funcionamiento sin conexión.

## Fuera de alcance

- Lógica de negocio que pertenece a los servicios.
- Generación de tokens de LiveKit: el cliente los recibe del backend.

## Contratos que consume

- [`contracts/openapi/`](../../contracts/openapi/)
- [`contracts/events/`](../../contracts/events/)

## Pendiente de decidir

- Implementación de la voz en el cliente móvil (LiveKit publica un SDK oficial para Flutter).

## Estado

Sin inicializar. El primer PR del responsable crea el proyecto (`flutter create`) con las versiones exactas de su entorno y actualiza este README con los pasos para compilar y ejecutar.
