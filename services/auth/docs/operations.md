# Operación de Auth

La configuración requerida y sus valores de referencia se encuentran en
[configuración](configuration.md). Los contratos contienen las garantías HTTP;
esta guía concentra los procedimientos de operación y diagnóstico.

## Claves JWT y JWKS

El JWKS es una instantánea pública validada al arrancar, ordenada por `kid`.
No se mezclan archivos durante una solicitud. El endpoint público permite GET y
HEAD, incluso sin disponibilidad de Redis o PostgreSQL; su contrato de caché
pertenece al [perfil de tokens](../../../contracts/auth/tokens.md).
No hay rotación automática ni recarga en caliente.

La rotación programada cada 90 días conserva el siguiente orden:

1. El operador aprovisiona una privada nueva fuera del repositorio, con `kid`
   nunca reutilizado, y prepara un JWKS con las dos públicas.
2. Todas las réplicas publican ese JWKS tras un reinicio controlado, conservando
   la privada y el `kid` anteriores para firmar. Se verifica el conjunto publicado.
3. La propagación espera al menos cinco minutos más el margen operativo acordado.
   El perfil propone cinco minutos adicionales; un fallo impide la activación.
4. El operador cambia privada y `kid` activos y reinicia las réplicas, conservando
   ambas públicas. Registra la última emisión y expiración con la clave saliente.
5. La pública anterior se retira solo después de vencer el último token emitido
   con ella, más tolerancia de reloj y margen. El cálculo incluye réplicas que
   hayan continuado firmando durante el cambio.

Los respaldos y el rollback deben conservar publicación y retención coherentes.
El compromiso de una clave requiere una política de emergencia independiente;
la caché pública impide prometer retirada instantánea. Los relojes deben estar
sincronizados. La comprobación mediante gateway de desarrollo puede realizarse con:

```bash
curl -i http://localhost:8080/.well-known/jwks.json
curl -I http://localhost:8080/.well-known/jwks.json
```

## Credenciales de servicio

Las identidades privadas son `core` y `files`. La configuración usa el
[archivo de ejemplo](../internal-credentials.example.json), cuyos marcadores
inválidos deben sustituirse mediante aprovisionamiento protegido.

Cada entrada contiene `enabled`, `permissions`, `active_sha256` y `previous`.
Los permisos admitidos son la lista vacía o `sessions:check`. `previous` es nulo
o contiene `sha256` y `expires_at` UTC. No se admiten hashes repetidos entre
posiciones o identidades, campos desconocidos ni permisos duplicados.

El consumidor recibe el secreto; Auth conserva SHA-256 del token ASCII completo,
incluido `svc_`, en hexadecimal minúsculo. La comparación es de tiempo constante.
Las credenciales son independientes de JWT, refresh y AMQP; no se distribuyen a
clientes móviles o de escritorio.

La rotación requiere reinicio de todas las réplicas:

1. El operador genera y distribuye una credencial nueva por el canal de secretos
   del consumidor correspondiente.
2. Un archivo completo y validable incorpora el hash nuevo como activo y el
   anterior con una caducidad UTC absoluta. Al arrancar, la convivencia restante
   no puede superar 15 minutos. El reemplazo del archivo es atómico.
3. Todas las réplicas se reinician y el consumidor cambia dentro de esa ventana.
   La fecha de caducidad no se recalcula por solicitud ni por reinicio.
4. En `expires_at`, la credencial anterior deja de aceptarse. Su eliminación
   posterior del archivo no la reactiva ni prolonga la convivencia.

La revocación anticipada retira el hash o deshabilita la identidad y requiere
reinicio coordinado. Una réplica con una instantánea anterior conserva su política
hasta reinicio o caducidad. El procedimiento exige un reloj UTC sincronizado.

## API interna HTTPS

El listener privado no pasa por Nginx público y usa exclusivamente la IP del
socket para autorizar CIDR. La validación se aplica antes de TLS y en HTTP.
El acceso remoto requiere red privada, CIDR específicos, DNS/SAN y confianza TLS
en Core y Files. El cliente verifica cadena, nombre y vigencia; validar PEM y
correspondencia de clave en Auth no sustituye esas verificaciones del cliente.

HTTP/1.1 admite una solicitud por conexión, hasta 32 cabeceras y un buffer de
16 KiB. Existen semáforos locales separados para conexiones y solicitudes. Las
cuotas por servicio, las reservas y el presupuesto previo por IP se comparten en
Redis. Los límites y errores observables se mantienen en el
[contrato interno](../../../contracts/auth/internal-api.md#autenticación-y-límites).

El pool privado usa el mismo PostgreSQL primario, en modo read-only y con
`statement_timeout`, con hasta `min(concurrencia_por_servicio, 10)` conexiones
por proceso. Ese presupuesto se suma al pool público al dimensionar réplicas.
Un timeout del futuro no garantiza cancelación SQL; el límite de sentencia
acota también el trabajo en PostgreSQL.

Las réplicas comparten primario lógico, base Redis, namespace y políticas.
La pérdida de estado o un failover que pierda escrituras puede reiniciar cuotas
y reservas. Redis Cluster/HA requiere revisión de persistencia y recuperación;
no se atribuye al semáforo local la garantía de un límite global.

La [plantilla de firewall privado](../../../infrastructure/firewall/auth-internal.nft.example)
es loopback-only y no se instala mediante `auth-firewall.service` ni los scripts
de desarrollo. Su revisión sintáctica no implica que está aplicada.

## Protección pública

Nginx aplica límites locales por IP antes de Auth, con zonas compartidas entre
sus workers, no entre instancias. Las tasas, `burst` y `nodelay` tienen su fuente
en [rutas](../../../gateway/nginx/routes/auth.conf) y
[servicios](../../../gateway/nginx/services/auth.conf). Los rechazos incluyen
`X-RateLimit-Layer: nginx` y `Retry-After: 60`; la verificación GET conserva HTML.

Auth usa token buckets distribuidos: los tokens se recargan progresivamente y
el periodo representa la recarga completa. Los valores configurables se mantienen
en [.env.example](../.env.example). Las rutas de contraseña usan políticas propias
según su [contrato](../../../contracts/auth/public-api.md#presupuestos).

El registro y el reenvío requieren Redis. La verificación conserva una excepción
ante indisponibilidad de Redis porque los fallos de código se limitan también en
PostgreSQL; un presupuesto explícitamente agotado sigue devolviendo 429. Login,
refresh, logout y contraseñas fallan con 503 cuando no puede comprobarse su
protección. Los presupuestos de destinatario y el cooldown persistente son controles
independientes; consumir presupuesto no garantiza entrega de correo.

La identidad pública solo acepta `X-Real-IP` desde las IP exactas configuradas en
`TRUSTED_PROXY_IPS`. Nginx sobrescribe esa cabecera y elimina `Forwarded` y
`X-Forwarded-For`. La API privada no utiliza esas cabeceras como identidad.

Las claves Redis por IP contienen direcciones. Los identificadores por correo
se derivan mediante HMAC; no son cifrado ni anonimización garantizada. La conexión
Redis se inicializa de forma diferida y se reutiliza. Los timeouts conservan una
categoría específica; el consumo no se repite automáticamente tras un fallo.

El firewall público permite el puerto 3001 desde Nginx por `br-auth` y rechaza
las demás conexiones, incluido loopback. `auth-firewall.service` carga
`/etc/team-collaboration-platform/auth.nft`; `active (exited)` es normal para esta
unidad oneshot. Cambiar el archivo del repositorio no actualiza por sí solo el
archivo instalado ni las reglas activas. El frente HTTP actual es de desarrollo;
producción requiere HTTPS.

## Correo y retención

Los transportes Mailpit y Resend comparten envío, con plantillas independientes
para registro y recuperación. El [contrato de correo](../../../contracts/auth/public-api.md#correo-de-recuperación)
conserva el texto aprobado y sus restricciones de contenido.

La recuperación admite hasta 32 tareas locales por réplica con plazas retenidas
11 segundos; el proveedor tiene un límite de 10 segundos. Los envíos ya aceptados
no son durables y no tienen reintento automático. La limpieza horaria conserva
intentos terminales o expirados al menos 24 horas y no modifica cuentas ni sesiones.
La rotación HMAC requiere coordinación y vuelve inutilizables códigos pendientes.

## Logs y diagnóstico

Los logs de Nginx usan nombres constantes de operación, estados, duración y
resultado de `limit_req`, sin URL completa, query, cuerpos ni cabeceras.
`error_log` permanece desactivado para evitar URLs sensibles. La retención Docker
es por tamaño (`10m`, tres archivos), no por días.

`ApiError` no muestra sus diagnósticos internos en `Debug` ni registra errores SQL
completos. El arranque evita reflejar errores del parser `.env` y diagnósticos de
conexión o migración. Las pruebas de `ApiError` comprueban la ausencia de esos diagnósticos en logs y
respuestas. Los proxies externos y la configuración de logs del despliegue requieren revisión propia.

| Señal | Interpretación |
|---|---|
| Nginx `REJECTED` | Rechazo del limitador del gateway |
| `PASSED` con upstream 429 | Rechazo del backend |
| Gateway 502 | Respuesta inválida o upstream inaccesible |
| `/health` | Disponibilidad del proceso |
| `/health/db` | Comprobación de PostgreSQL |
| `/health/redis` | PING acotado con la conexión del limitador |

Desde la VM, el diagnóstico puede usar:

```bash
sudo systemctl status auth-firewall.service --no-pager
sudo nft list table inet auth_guard
sudo ss -ltnp 'sport = :3001'
curl --noproxy '*' -i --connect-timeout 2 http://127.0.0.1:3001/health
curl --noproxy '*' -i http://127.0.0.1:8080/health
curl --noproxy '*' -i http://127.0.0.1:8080/health/redis
```

Con el firewall público instalado, el primer curl debe fallar. Si el acceso
directo funciona y el gateway responde 502, corresponde revisar upstream, bridge
y reglas. Ningún health check aislado acredita readiness conjunta. `cargo test`
no mantiene un servidor Auth en ejecución.
