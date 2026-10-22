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

La configuración actual enruta registro, verificación, reenvío y health de Auth,
con presupuestos públicos por IP y registros sin credenciales ni query sensible.

La segunda entrega de #35 agrega la ruta exacta `/.well-known/jwks.json` para
GET/HEAD público. Conserva el `Cache-Control` de Auth (300 segundos), sin consumir
presupuestos de registro/verificación/reenvío. No se modifica firewall ni acceso
directo a Auth. La configuración actual escucha HTTP de desarrollo; no configura
certificados HTTPS de producción.

Prueba aislada de configuración y enrutamiento, con Nginx instalado:

```sh
python3 scripts/test-auth-jwks-gateway.py --nginx /usr/sbin/nginx
```

Ejecutar desde la raíz. Usa configuración temporal, backend stub y loopback;
comprueba GET/HEAD, caché, límites existentes y 404. No demuestra integración
con un Auth conectado a PostgreSQL/Redis. Ver [operación de claves](../../services/auth/TOKENS.md).
