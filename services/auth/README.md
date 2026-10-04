# Auth Service

**Responsable principal:** `@tuzc0`

## Responsabilidad

- Registro, inicio y cierre de sesión.
- Recuperación de contraseña y verificación de correo.
- Gestión de sesiones y emisión de tokens firmados con EdDSA.
- Perfil de usuario: este servicio es el dueño de los datos de usuario.

## Stack

- Rust con Axum, Tokio, Tower y SQLx.
- PostgreSQL, en su propia base de datos.
- Redis para sesiones y rate limiting cuando corresponda.
- Argon2id para contraseñas.

## Fuera de alcance

- Grupos, roles dentro de grupos, proyectos y demás dominio de Core.
- Archivos (foto de perfil incluida): se guardan mediante el File Service.

## Contratos

- Expone: [`contracts/openapi/auth.yaml`](../../contracts/openapi/)
- Los demás servicios validan los tokens con la clave pública; nunca llaman a Auth para validar cada petición.

## Base de datos

Las migraciones viven dentro de este servicio (SQLx). Ver [`database/README.md`](../../database/README.md).

## Estado

Sin inicializar. El primer PR del responsable crea el proyecto (`cargo new`) con las versiones exactas de su entorno y actualiza este README con los pasos para compilar y ejecutar.
