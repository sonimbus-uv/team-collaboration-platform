# Rate limiting de Auth

## Alcance

La protección cubre:

- `POST /auth/register`.
- `POST /auth/resend-verification`.
- `GET` y `POST /auth/verify-email`.

`GET /health`, `GET /health/db`, `GET /health/redis` y `HEAD /auth/verify-email` no consumen
presupuesto del limitador de Auth.

`HEAD /auth/verify-email` responde `405` sin confirmar el correo.

## Capas de protección

### Nginx

Aplica límites por IP y por operación antes de llegar a Auth.

Utiliza zonas locales de memoria compartida entre sus workers.
Estas zonas no comparten presupuesto entre instancias diferentes de Nginx.

La configuración actual es:

| Operación | Tasa | Burst |
|---|---|---|
| Registro | 60 solicitudes/minuto | 12 |
| Reenvío | 30 solicitudes/minuto | 6 |
| Verificación | 120 solicitudes/minuto | 30 |

Se utiliza `nodelay`: las solicitudes admitidas dentro del burst
continúan sin una espera deliberada.

Los rechazos propios del gateway responden `429` e incluyen
`X-RateLimit-Layer: nginx` y `Retry-After: 60` (espera conservadora).

Para GET de verificación, la respuesta es HTML. Para las operaciones
de API, es `application/problem+json`.

### Auth y Redis

Auth aplica un token bucket distribuido mediante Redis.

Cada bucket tiene una capacidad máxima y un periodo de recarga completa.
Los tokens se recuperan progresivamente; no se trata de una ventana
que reinicia todo el presupuesto al terminar el periodo.

Valores predeterminados:

| Presupuesto | Capacidad | Recarga completa |
|---|---|---|
| Registro por IP | 6 | 60 segundos |
| Reenvío por IP | 3 | 60 segundos |
| Verificación por IP | 20 | 60 segundos |
| Envío por destinatario | 3 | 3600 segundos |

GET y POST de verificación comparten el mismo bucket por IP.

Las réplicas de Auth comparten presupuesto cuando utilizan el mismo
Redis, base de datos, claves y configuración.

Estos valores son una configuración inicial y deben revisarse con
tráfico observado, especialmente para usuarios que comparten IP pública.
Las variables correspondientes se documentan en [`.env.example`](.env.example).

## Identificación del cliente

Auth utiliza la IP de la conexión directa.

Solo acepta `X-Real-IP` cuando la conexión procede de una IP incluida
explícitamente en `TRUSTED_PROXY_IPS`.

En la configuración actual de desarrollo:

```env
TRUSTED_PROXY_IPS=172.30.250.2
REQUIRE_TRUSTED_PROXY=true
```

Nginx sobrescribe `X-Real-IP` con la dirección que observa y elimina
`X-Forwarded-For` y `Forwarded` antes de enviar la solicitud a Auth.

La confianza corresponde a la IP de Nginx que se conecta a Auth.
No corresponde a la IP del cliente final.

## Claves en Redis

Los presupuestos por IP utilizan:

```text
auth:rl:v1:register:ip:{ip}
auth:rl:v1:resend:ip:{ip}
auth:rl:v1:verify:ip:{ip}
```

El presupuesto por destinatario utiliza:

```text
auth:rl:v1:email-send:recipient:{hmac}
```

El correo se normaliza y se transforma mediante HMAC-SHA256.
El correo en texto plano no forma parte de esta clave.

Todas las réplicas deben compartir `AUTH_RECIPIENT_HMAC_SECRET`.
Cambiar este secreto genera claves nuevas y deja de consultar los
presupuestos anteriores.

Las claves por IP sí contienen la dirección IP.

## Ejecución del bucket

`rate_limit.lua` realiza en una ejecución atómica:

1. Obtención del tiempo desde Redis.
2. Lectura y recarga del presupuesto.
3. Consumo de un token si hay disponibilidad.
4. Cálculo de la espera si no hay disponibilidad.
5. Actualización del estado y de su expiración.

La expiración permite eliminar el estado cuando el bucket ya habría
recuperado su capacidad completa.

Un timeout del cliente no demuestra que el script no se ejecutó.
Por ello no se reintenta automáticamente el consumo.

## Presupuesto por destinatario y cooldown

El presupuesto por destinatario se aplica únicamente a reenvíos elegibles.
El registro inicial no lo consume; los registros duplicados no envían correo.

Este presupuesto cuenta intentos admitidos hacia una operación que
puede enviar correo. No representa un contador de correos entregados.

La cuenta se bloquea en PostgreSQL y se comprueban su estado y el cooldown
antes de consumir presupuesto. Las cuentas inexistentes, verificadas o
inactivas y las solicitudes durante el cooldown no lo consumen.
Si Redis rechaza el consumo o falla, se revierte la transacción sin crear
un token y se responde `202`. No se devuelve presupuesto si después falla
la base de datos o el proveedor: un timeout no demuestra que no hubo consumo.

El cooldown de PostgreSQL de 60 segundos sigue siendo una protección
independiente.

Un `202` de reenvío no garantiza que se haya enviado otro correo.
Los errores del proveedor se registran sin cambiar ese estado HTTP.
Esto no garantiza tiempos de respuesta indistinguibles; el registro
conserva su respuesta `409` para cuentas existentes.

Reenviar no invalida los enlaces anteriores: conservan su caducidad original.
Confirmar uno verifica la cuenta e invalida los demás enlaces pendientes
en la misma transacción. Los tokens solo se persisten como hashes.

La confirmación del token no consume presupuesto por destinatario.

## Respuestas y fallos

| Situación | Respuesta |
|---|---|
| Presupuesto por IP de Auth agotado | `429` con `Retry-After` |
| Redis no disponible para registro o reenvío por IP | `503` |
| Redis no disponible para verificación por IP | Continúa la verificación |
| Presupuesto por destinatario agotado o no disponible | `202` sin generar token |
| Presupuesto de Nginx agotado | `429` con `X-RateLimit-Layer: nginx` y `Retry-After: 60` |
| Dirección ausente o inválida desde un proxy confiable | `500` |
| Token inválido admitido por los limitadores | `400` |
| HEAD de verificación | `405` |

Registro y reenvío fallan con `503` si falla su comprobación por IP.
GET y POST de verificación continúan cuando esa comprobación de Redis falla:
el token tiene 256 bits aleatorios y Nginx mantiene su límite independiente.
La validación del token y la transacción de PostgreSQL siguen siendo obligatorias.
Un rechazo explícito de Redis por presupuesto agotado sigue produciendo `429`.

GET de verificación conserva respuestas HTML.
Las operaciones de API utilizan Problem Details.

Un `502` del gateway indica un problema al obtener una respuesta válida
del upstream; no es un rechazo del rate limit.

`GET /health` comprueba que el proceso responde; `GET /health/db` comprueba
PostgreSQL. `GET /health/redis` ejecuta PING con los mismos timeouts y conexión
del limitador: responde `200` o `503`. No es una comprobación conjunta de readiness.

## Protección del acceso directo

El firewall protege el puerto 3001:

- Permite a Nginx desde `172.30.250.2` por `br-auth`.
- Rechaza las demás conexiones dirigidas al puerto, incluido loopback.

La red `auth_gateway` fija el nombre del bridge y la IP de Nginx.

`auth-firewall.service` carga las reglas desde:

```text
/etc/team-collaboration-platform/auth.nft
```

El servicio es oneshot: `active (exited)` es su estado esperado después
de aplicar correctamente las reglas.

Los cambios del archivo del repositorio deben instalarse también en
la ruta de `/etc` antes de recargar el servicio.

El aprovisionamiento instala nftables, las reglas y la unidad; aplica y
recarga la protección. Un drop-in de Docker requiere `auth-firewall.service`
y ordena su ejecución antes de Docker. También comprueba la IP real de Nginx.
No ejecutar el aprovisionamiento deja el despliegue existente sin actualizar.

## Registros

Nginx registra operaciones mediante nombres constantes, estados HTTP,
duraciones y resultado de `limit_req`.

El formato no incluye URL completa, parámetros, cuerpos ni cabeceras.
`error_log` permanece desactivado para evitar registrar URLs con tokens.

Interpretación:

- `REJECTED`: rechazo del limitador de Nginx.
- `PASSED` con `upstream_status` igual a `429`: el upstream respondió `429`.
- Estado vacío del limitador: no se aplicó en esa solicitud.

Docker utiliza `json-file` con `max-size=10m` y `max-file=3` para Nginx.
La retención está limitada por tamaño, no por días.

## Pruebas automatizadas

Desde `/vagrant/services/auth`:

```bash
export CARGO_TARGET_DIR="$HOME/.cargo-target/auth-service"

cargo test --locked

cargo test --locked redis_connection::tests:: -- --ignored
```

Se necesitan PostgreSQL y Redis de desarrollo disponibles y las variables
de conexión configuradas en el `.env` local.
Las pruebas SQLx requieren permisos para crear bases de datos temporales.
El workflow `.github/workflows/auth.yml` proporciona PostgreSQL y Redis y
ejecuta `cargo test --locked -- --include-ignored`; sus fallos no se omiten.

## Diagnóstico básico

```bash
sudo systemctl status auth-firewall.service --no-pager
sudo nft list table inet auth_guard
sudo ss -ltnp 'sport = :3001'

# Debe fallar: el acceso directo está bloqueado.
curl --noproxy '*' -i --connect-timeout 2 http://127.0.0.1:3001/health
curl --noproxy '*' -i http://127.0.0.1:8080/health
curl --noproxy '*' -i http://127.0.0.1:8080/health/redis

docker compose --env-file /vagrant/infrastructure/.env \
    -f /vagrant/infrastructure/compose.yaml logs --since 5m nginx
```

Si el acceso directo funciona y el gateway devuelve `502`, revisar
la dirección del upstream, la red y los contadores del firewall.

Si ambos accesos fallan, comprobar primero que Auth está ejecutándose.
`cargo test` no mantiene el servidor iniciado.
