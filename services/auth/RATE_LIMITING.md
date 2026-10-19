# Rate limiting de Auth

## Alcance

La protección cubre:

- `POST /auth/register`.
- `POST /auth/resend-verification`.
- `GET` y `POST /auth/verify-email`.

`GET /health`, `GET /health/db` y `HEAD /auth/verify-email` no consumen
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
`X-RateLimit-Layer: nginx`.

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

Registro y reenvío comparten el presupuesto por destinatario.

Este presupuesto cuenta intentos admitidos hacia una operación que
puede enviar correo. No representa un contador de correos entregados.

Un reenvío que no envía correo por el cooldown puede consumir presupuesto.
No se devuelven tokens por errores o por operaciones sin envío.

El cooldown de PostgreSQL de 60 segundos sigue siendo una protección
independiente.

Un `202` de reenvío no garantiza que se haya enviado otro correo.

La confirmación del token no consume presupuesto por destinatario.

## Respuestas y fallos

| Situación | Respuesta |
|---|---|
| Presupuesto de Auth agotado | `429` con `Retry-After` |
| Protección de Auth no disponible | `503` |
| Presupuesto de Nginx agotado | `429` con `X-RateLimit-Layer: nginx` |
| Token inválido admitido por los limitadores | `400` |
| HEAD de verificación | `405` |

Auth aplica cierre ante fallos de Redis en las operaciones protegidas:
responde `503` en lugar de permitirlas sin protección.

GET de verificación conserva respuestas HTML.
Las operaciones de API utilizan Problem Details.

Un `502` del gateway indica un problema al obtener una respuesta válida
del upstream; no es un rechazo del rate limit.

Los endpoints de salud existentes no comprueban la disponibilidad de Redis.

## Protección del acceso directo

El firewall protege el puerto 3001:

- Permite acceso por loopback para desarrollo.
- Permite a Nginx desde `172.30.250.2` por `br-auth`.
- Rechaza las demás conexiones dirigidas al puerto.

La red `auth_gateway` fija el nombre del bridge y la IP de Nginx.

`auth-firewall.service` carga las reglas desde:

```text
/etc/team-collaboration-platform/auth.nft
```

El servicio es oneshot: `active (exited)` es su estado esperado después
de aplicar correctamente las reglas.

Los cambios del archivo del repositorio deben instalarse también en
la ruta de `/etc` antes de recargar el servicio.

La unidad ordena su ejecución antes de Docker, pero no convierte
el éxito del firewall en un requisito obligatorio para iniciar Docker.

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

## Diagnóstico básico

```bash
sudo systemctl status auth-firewall.service --no-pager
sudo nft list table inet auth_guard
sudo ss -ltnp 'sport = :3001'

curl --noproxy '*' -i http://127.0.0.1:3001/health
curl --noproxy '*' -i http://127.0.0.1:8080/health

docker compose --env-file /vagrant/infrastructure/.env \
    -f /vagrant/infrastructure/compose.yaml logs --since 5m nginx
```

Si el acceso directo funciona y el gateway devuelve `502`, revisar
la dirección del upstream, la red y los contadores del firewall.

Si ambos accesos fallan, comprobar primero que Auth está ejecutándose.
`cargo test` no mantiene el servidor iniciado.
