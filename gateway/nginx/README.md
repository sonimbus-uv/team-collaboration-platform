# API Gateway

**Responsable principal:** `@tuzc0`  
**Apoyo:** `@Maple-M136279841` (módulo compartido)

## Responsabilidad

Punto único de entrada para los clientes: enruta las peticiones REST y las conexiones WebSocket hacia Auth, Core y el File Service. En despliegue, también la terminación TLS y los límites de peticiones.

## Stack

- Nginx, ejecutado como servicio en `infrastructure/compose.yaml`.

## Fuera de alcance

- Lógica de negocio y validación de tokens dentro del gateway, mientras no se decida lo contrario mediante un ADR.

## Estado

Sin configuración. El primer PR agrega `nginx.conf` y el servicio en Compose.
