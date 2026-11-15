# API Gateway

**Responsable principal:** `@tuzc0`  
**Apoyo:** `@Maple-M136279841` (módulo compartido)

## Responsabilidad

Punto único de entrada para los clientes: enruta las peticiones REST y las conexiones WebSocket hacia Auth, Core, el File Service y el Report Service, y gRPC (`grpc_pass`) si se elige para archivos. También hace la terminación TLS (CA local en desarrollo), los límites de peticiones (`limit_req`) y el tamaño máximo de subida (`client_max_body_size`).

## Stack

- Nginx, ejecutado como servicio en `infrastructure/compose.yaml`.

## Fuera de alcance

- Lógica de negocio y validación de tokens: la versión libre de Nginx no valida JWT, así que cada servicio verifica el token (D-02).

## Estado

La tercera entrega agrega POST `/api/v1/auth/login`, `/api/v1/auth/refresh` y
`/api/v1/auth/logout`, con presupuestos edge separados y Problem Details estable.
El login permanece deshabilitado por defecto en Auth mientras #17 siga abierto.
Ver [configuración y límites propuestos](../../services/auth/docs/architecture.md#sesiones-y-transacciones).

La configuración actual enruta registro, verificación, reenvío y health de Auth,
con presupuestos públicos por IP y registros sin credenciales ni query sensible.

La segunda entrega de #35 agrega la ruta exacta `/.well-known/jwks.json` para
GET/HEAD público. Conserva el `Cache-Control` de Auth (300 segundos), sin consumir
presupuestos de registro/verificación/reenvío. No se modifica firewall ni acceso
directo a Auth. El Compose base escucha HTTP de desarrollo. El override
`infrastructure/compose.auth-tls.yaml` agrega HTTPS en loopback `8443` con TLS 1.2/1.3
y certificados locales separados de la API interna. La preparación y el túnel a
Windows se describen en [desarrollo de Auth](../../services/auth/docs/development.md#https-del-gateway-y-callback-google).
No configura certificados HTTPS de producción.

Prueba aislada de configuración y enrutamiento, con Nginx instalado:

```sh
python3 scripts/auth/tests/test-auth-jwks-gateway.py --nginx /usr/sbin/nginx
```

Ejecutar desde la raíz. Usa configuración temporal, backend stub y loopback;
comprueba GET/HEAD, caché, límites existentes y 404. No demuestra integración
con un Auth conectado a PostgreSQL/Redis. Ver [operación de claves](../../services/auth/docs/operations.md#claves-jwt-y-jwks).

Las rutas exactas Google `attempts`, `authorize`, `callback` y `exchange` usan
el presupuesto compartido `auth_google` (60 solicitudes/minuto, burst 12 por IP).
Los POST limitan cuerpo a 4 KiB; callback espera hasta 15 segundos para cubrir
el intercambio OIDC acotado. El log registra únicamente nombres de operación,
sin query, ticket, código, cookie ni secreto de intento. El servicio conserva
su bandera Google deshabilitada hasta el aprovisionamiento descrito en
[configuración](../../services/auth/docs/configuration.md#configuración-del-adaptador-google).

Vinculación (`google/link-attempts`) y lectura de conexiones (`connections`) comparten
la zona Google. `reauthenticate` tiene zona propia de 6 solicitudes/minuto y burst 6.
Los nuevos POST conservan máximo de 4 KiB y declaran explícitamente los manejadores
JSON de 413 y 429 para mantener ambos al configurar errores por location.

Contacto usa rutas exactas para lectura, solicitud, reenvío y confirmación. Solicitud
y reenvío comparten `auth_contact_send` (6r/m, burst 6); lectura usa `auth_contact_read`
(30r/m, burst 10) y confirmación `auth_contact_verify` (20r/m, burst 10). Los POST
limitan cuerpo a 4 KiB; solicitud/reenvío esperan hasta 15 segundos. El manejador
común `@auth_body_too_large` mantiene respuestas JSON 413 para contraseñas y contacto.
El bucket de los mapas se amplía a 128 bytes para las nuevas claves de ruta.
