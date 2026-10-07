# API Gateway

**Responsable principal:** `@tuzc0`  
**Apoyo:** `@Maple-M136279841` (módulo compartido)

## Responsabilidad

Punto único de entrada para los clientes: enruta las peticiones REST y las conexiones WebSocket hacia Auth, Core, el File Service y el Report Service, y gRPC (`grpc_pass`) si se elige para archivos. También hace la terminación TLS (`mkcert` en desarrollo), los límites de peticiones (`limit_req`) y el tamaño máximo de subida (`client_max_body_size`).

## Stack

- Nginx, ejecutado como servicio en `infrastructure/compose.yaml`.

## Fuera de alcance

- Lógica de negocio y validación de tokens: la versión libre de Nginx no valida JWT, así que cada servicio verifica el token (D-02).

## Estado

Sin configuración. El primer PR agrega `nginx.conf` y el servicio en Compose.
