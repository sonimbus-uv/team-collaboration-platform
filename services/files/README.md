# File Service

**Responsable principal:** `@Maple-M136279841`

## Responsabilidad

Subida, descarga y eliminación de archivos (imágenes, documentos, audios y adjuntos) almacenados en MinIO, con validación de tipo y tamaño.

## Stack

- Elixir.
- MinIO como almacenamiento de objetos.

## Fuera de alcance

- Decidir quién puede ver un archivo según grupos o proyectos: esa autorización de dominio pertenece a Core.

## Contratos

- Expone: [`contracts/openapi/files.yaml`](../../contracts/openapi/)

## Pendiente de decidir

- Protocolo de transferencia de archivos: REST o gRPC.
- Dónde se guarda la metadata de cada archivo.

## Estado

Sin inicializar. El primer PR del responsable crea el proyecto (`mix new` o `mix phx.new`, según el protocolo elegido) con las versiones exactas de su entorno y actualiza este README con los pasos para compilar y ejecutar.
