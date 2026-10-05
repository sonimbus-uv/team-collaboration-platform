# Bases de datos

## Reglas

- Cada servicio es dueño de sus datos. Auth y Core usan bases de datos separadas en la misma instancia de PostgreSQL, y ningún servicio lee directamente la base de otro.
- Las migraciones de PostgreSQL viven dentro de cada servicio, porque sus herramientas las esperan ahí (SQLx en Auth, Ecto en Core).
- Esta carpeta guarda lo que no pertenece a un solo servicio.

| Carpeta | Contenido |
|---|---|
| [`postgres/`](postgres/) | Scripts de inicialización: creación de bases de datos y usuarios por servicio. |
| [`scylla/`](scylla/) | Esquemas CQL (keyspaces y tablas) de ScyllaDB. |

Responsables: los scripts de inicialización de PostgreSQL, `@tuzc0` (Infraestructura); las migraciones, cada servicio (`@tuzc0` en Auth, `@Maple-M136279841` en Core); los esquemas de ScyllaDB, `@Maple-M136279841` (Core).
