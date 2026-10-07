# Report Service

**Responsable principal:** `@roluva`

## Responsabilidad

Reportes básicos de actividad de un grupo para su administrador (CU-29) y su exportación a PDF o CSV (CU-30, prioridad baja):

- mensajes por periodo;
- participación por miembro;
- tareas por estado.

El servicio no consulta los datos de otros servicios: construye sus propios agregados consumiendo eventos de RabbitMQ (`message.created`, `task.updated`, eventos de membresía) y los guarda en su esquema.

## Stack

- Lenguaje: **por decidir** (propuesta: Elixir, para que el backend quede en Rust solo para Auth). Ver las decisiones pendientes en [`docs/decisions/`](../../docs/decisions/README.md#decisiones-pendientes).
- PostgreSQL, esquema `reports` con su propio usuario (D-13).
- RabbitMQ como consumidor (D-04).

## Requisitos

- RF-REP-01, RF-REP-02.
- RNF-DIS-01: si este servicio falla, el resto del sistema sigue operando.
- RNF-SEG-03: solo el administrador del grupo consulta sus reportes; se verifica con `checkAccess` de Core.
- El periodo consultado no puede exceder un rango máximo (valor pendiente).

Ver la trazabilidad completa en [`docs/requirements/`](../../docs/requirements/README.md#report-service).

## Fuera de alcance

- Decidir roles y permisos: pertenece a Core.
- Guardar mensajes o tareas originales: solo agregados.

## Contratos

- Expone: `contracts/openapi/reports.yaml`.
- Consume: eventos de [`contracts/events/`](../../contracts/events/).

## Estado

Sin inicializar. Cuando se decida el lenguaje, el primer PR del responsable crea el proyecto con las versiones exactas de su entorno y actualiza este README con los pasos para compilar y ejecutar (T-10).
