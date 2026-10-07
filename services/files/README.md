# File Service

**Responsable principal:** `@Maple-M136279841`

## Responsabilidad

Recepción, validación de tipo y tamaño, almacenamiento, vistas previas y entrega de archivos e imágenes compartidos en los canales (CU-23 y CU-24). Las subidas incompletas quedan en estado pendiente y se limpian periódicamente.

Ver la trazabilidad completa en [`docs/requirements/`](../../docs/requirements/README.md#file-service).

## Stack

- Elixir (D-03).
- MinIO para el contenido de los archivos.
- PostgreSQL, esquema `files` con su propio usuario (D-13), para los metadatos: nombre, tipo, tamaño, autor, canal y estado.

## Fuera de alcance

- Decidir quién puede ver un archivo según grupos y canales: esa autorización pertenece a Core y se consulta con `checkAccess`.
- Publicar el mensaje asociado al archivo: lo hace Core.
- Fotos de perfil: se eligen de un paquete predefinido en los clientes (D-06) y no pasan por este servicio.

## Contratos

- Expone: `contracts/openapi/files.yaml` (o `contracts/proto/` si se elige gRPC).
- Consume: `checkAccess` de Core.

## Pendiente de decidir

- Protocolo de transferencia: gRPC con streaming o REST con subida por partes y `Range`. Se resuelve con una PoC de 50 MB por bloques, de JavaFX a Elixir pasando por Nginx.
- Tamaño máximo y tipos de archivo permitidos.

## Estado

Sin inicializar. El primer PR del responsable crea el proyecto (`mix new` o `mix phx.new`, según el protocolo elegido) con las versiones exactas de su entorno y actualiza este README con los pasos para compilar y ejecutar.
