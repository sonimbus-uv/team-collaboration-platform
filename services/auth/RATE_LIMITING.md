# Rate limiting de Auth

## Alcance

La protección cubre:

- `POST /auth/register`.
- `POST /auth/resend-verification`.
- `GET` y `POST /auth/verify-email`.
- `POST /api/v1/auth/login`, `/api/v1/auth/refresh` y `/api/v1/auth/logout`.
  Sus presupuestos y contratos se detallan en [SESSIONS.md](SESSIONS.md).

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
| Login | 60 solicitudes/minuto | 12 |
| Refresh | 120 solicitudes/minuto | 20 |
| Logout | 120 solicitudes/minuto | 20 |

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
| Envío por destinatario (registro y reenvío) | 10 | 600 segundos |
| Login por IP | 6 | 60 segundos |
| Refresh por IP | 30 | 60 segundos |
| Logout por IP | 30 | 60 segundos |
| Login por correo identificado mediante HMAC | 10 | 900 segundos |

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
Las claves de login por correo usan `auth:rl:v1:login:account:{hmac}` y un
dominio HMAC distinto del usado por reenvío. Ambos normalizan trim/lowercase.
HMAC es un identificador con secreto, no cifrado ni anonimización garantizada.

`AuthRateLimiter` conserva las políticas en memoria. Inicializa su conexión
gestionada de Redis de forma diferida mediante `OnceCell` en el primer consumo
o health check y reutiliza la inicialización exitosa. Un fallo al inicializar no
queda guardado como conexión válida. Los timeouts de conexión/operación son
configurables; el presupuesto restante vive en Redis, no en AppState.

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

Desde la solución local de #17, registro y reenvío comparten este presupuesto.
El intento liga correo, hash de contraseña y MAC del código. Ver
[contrato vigente](../../contracts/registration.md).

Este presupuesto cuenta intentos admitidos hacia una operación que
puede enviar correo. No representa un contador de correos entregados.

El reenvío comprueba elegibilidad/cooldown antes de consultar el presupuesto,
y revalida bajo bloqueo antes de actualizar la MAC. Redis se consulta fuera de
la transacción. Si rechaza el consumo, responde 429; si falla, 503. No devuelve
presupuesto si después falla SQL, otro proceso gana la carrera o falla el
proveedor: un timeout no demuestra que no hubo consumo. Registro confirma
disponibilidad de cuenta antes de consultar presupuesto; la creación revalida
el cupo de cinco pendientes y el estado bajo bloqueo por correo.

El cooldown de PostgreSQL de 60 segundos sigue siendo una protección
independiente.

Un `202` de reenvío no garantiza entrega; `verification_email_sent` comunica
si el proveedor aceptó el envío. Un fallo conserva el intento y su cooldown.
Esto no garantiza tiempos de respuesta indistinguibles; el registro
conserva su respuesta `409` para cuentas existentes.

Reenviar invalida el código anterior del mismo intento, conserva vencimiento
y contador de fallos; máximo tres reenvíos. Crear otro intento no invalida los
existentes. Confirmar uno instala su contraseña e invalida los demás.

Confirmar no consume presupuesto de correo: cada intento permite cinco fallos
persistidos en PostgreSQL. Código/intento vencen a los diez minutos.

## Respuestas y fallos

| Situación | Respuesta |
|---|---|
| Presupuesto por IP de Auth agotado | `429` con `Retry-After` |
| Redis no disponible para registro o reenvío por IP | `503` |
| Redis no disponible para verificación por IP | Continúa la verificación |
| Presupuesto por destinatario agotado / no disponible | `429` / `503` |
| Presupuesto de Nginx agotado | `429` con `X-RateLimit-Layer: nginx` y `Retry-After: 60` |
| Dirección ausente o inválida desde un proxy confiable | `500` |
| Intento/código inválido admitido por los limitadores | `400` genérico |
| GET antiguo admitido por los limitadores | `410` informativo, sin activar |
| HEAD de verificación | `405` |

Registro y reenvío fallan con `503` si falla su comprobación por IP.
GET y POST de verificación continúan cuando esa comprobación de Redis falla:
la confirmación tiene un límite persistido de cinco fallos por intento en
PostgreSQL y Nginx mantiene su límite independiente. El código de ocho dígitos
está protegido con HMAC; nunca se confía solo en el límite IP. GET no activa nada.
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

Esto describe los registros de Nginx. En Auth, el manejo genérico de
`ApiError::Database` registra el error SQLx con Debug; los detalles de PostgreSQL
pueden incluir valores de campos. Las rutas de sesión y las nuevas rutas de
registro/verificación/reenvío usan mapeos distintos sin ese detalle; sus errores
de registro también tienen Debug redactado. El mapeo genérico sigue existiendo
para otros caminos, por lo que no se acredita redacción universal del servicio.

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

cargo test --locked infrastructure::redis_connection::tests:: -- --ignored
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
