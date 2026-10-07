# Cliente móvil

**Responsable principal:** `@roluva`

## Responsabilidad

Aplicación móvil de Sonimbus: interfaz de usuario, funcionamiento sin conexión y comunicación con el backend a través del gateway (REST y WebSocket). Incluye la voz con LiveKit.

Prioriza comunicación, consulta y participación básica (RES-05); la administración detallada se concentra en el escritorio (RES-06). El alcance propuesto por caso de uso está en [`docs/requirements/`](../../docs/requirements/README.md#cliente-móvil).

## Stack

- Flutter y Dart.
- SQLite mediante Drift para almacenamiento local y funcionamiento sin conexión.
- SDK oficial de LiveKit para Flutter (D-01).

## Fuera de alcance

- Lógica de negocio que pertenece a los servicios (permisos, validaciones de dominio).
- Generación de tokens de LiveKit: el cliente los recibe de Core (D-15).
- Notificaciones push: fase posterior.

## Contratos que consume

- [`contracts/openapi/`](../../contracts/openapi/)
- [`contracts/events/`](../../contracts/events/)

## Estado

Sin inicializar. El primer PR del responsable crea el proyecto (`flutter create`) con las versiones exactas de su entorno y actualiza este README con los pasos para compilar y ejecutar.
