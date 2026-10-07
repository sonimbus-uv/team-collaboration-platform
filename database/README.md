# Bases de datos

## Reglas

- Cada servicio es dueño de sus datos. Todos usan la misma instancia y la misma base de datos de PostgreSQL, pero cada uno tiene **su propio esquema y su propio usuario** (`auth`, `core`, `files`, `reports`), y ningún servicio lee el esquema de otro (D-13).
- Las migraciones de PostgreSQL viven dentro de cada servicio, porque sus herramientas las esperan ahí (SQLx en Auth, Ecto en Core).
- Esta carpeta guarda lo que no pertenece a un solo servicio.

| Carpeta | Contenido | Responsable |
|---|---|---|
| [`postgres/`](postgres/) | Scripts de inicialización: creación de esquemas y usuarios por servicio | `@tuzc0` |
| [`scylla/`](scylla/) | Esquemas CQL (*keyspaces* y tablas) de ScyllaDB (D-05) | `@Maple-M136279841` |

Las migraciones son responsabilidad de cada servicio: `@tuzc0` en Auth, `@Maple-M136279841` en Core y Files, y `@roluva` en Reports.

El modelo de datos de cada servicio está documentado en [`docs/database/`](../docs/database/).
