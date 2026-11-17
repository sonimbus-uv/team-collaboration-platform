# Comprobación interna de sesiones v1

Auth es la autoridad de sesiones en PostgreSQL. Core y Files consultan mediante
HTTPS y JSON por red privada, sin pasar por Nginx público ni acceder al esquema
`auth`. El cliente verifica la cadena TLS y el hostname del destino.

El [OpenAPI interno](../openapi/auth-internal.yaml) define rutas, estructuras y errores.
La implementación requiere activación explícita y no acredita despliegue ni
interoperabilidad. La [arquitectura](../../services/auth/docs/architecture.md#consulta-autoritativa-de-sesiones)
describe la consulta de dominio; la [operación](../../services/auth/docs/operations.md#api-interna-https),
los procedimientos de red y credenciales.

El [diseño Google/contacto aprobado](public-api.md#google-y-correo-de-contacto)
se refleja en la elegibilidad por método incorporada con `0011`: las sesiones
Google asociadas a una identidad de la misma cuenta pueden carecer de contacto
verificado. Las sesiones locales pendientes siguen rechazadas. El formato de
respuesta se conserva y el login Google requiere activación explícita e independiente.

## Autenticación y límites

`Authorization: Bearer svc_<secreto>` contiene un token opaco de 32 bytes aleatorios
codificados en base64url sin padding, prefijado por `svc_`. Tiene 47 caracteres y
se asocia a `core` o `files` con permiso `sessions:check`. No es JWT, refresh ni
credencial AMQP. La identidad no procede del cuerpo. La rotación admite una
credencial anterior durante hasta 15 minutos y requiere reinicio de todas las
réplicas; la distribución corresponde al canal protegido de secretos.

Los límites iniciales requieren medición antes de atribuirles capacidad productiva:

| Control | Alcance | Predeterminado |
|---|---|---|
| Pares consultados | Redis, por identidad entre réplicas | Capacidad 100 y recarga de 100 pares/s |
| Consultas simultáneas autenticadas | Reservas Redis por identidad | 20 |
| Vida de reserva | Reloj Redis | 2 s; al menos doble del presupuesto HTTP |
| Solicitudes previas a autenticar | Redis por IP del socket | Capacidad 10 y recarga de 10 solicitudes/s |
| Conexiones y solicitudes HTTP | Dos semáforos locales por proceso | 50 cada uno |
| Cuerpo | Por solicitud | 64 KiB |
| Lote | Por solicitud | 1–100 pares distintos |
| Solicitud HTTP | Local, incluye autenticación, Redis, SQL y liberación | 1 s |
| Adquisición y consulta SQL | Local | 500 ms |
| Redis conexión / operación | Local | 200 / 100 ms |
| TLS / cabeceras | Por conexión | 300 / 1000 ms |

La cuota cuenta pares, no lotes. Los token buckets admiten ráfagas hasta su
capacidad; no son máximos de ventana deslizante. Las cuotas son independientes
del límite público. Su agotamiento devuelve 429 con Retry-After; los fallos de
infraestructura o reserva devuelven 503, nunca una autorización fabricada.
No existe reintento automático. Los semáforos locales no constituyen un límite
global entre réplicas. Un rechazo previo a HTTP puede cerrar el transporte sin
una respuesta de sesión. La liberación de una reserva no devuelve cuota consumida.

## Consulta y errores

POST individual y lote reciben pares `sid`, `account_id` UUID tomados del JWT
ya validado (`sub` → `account_id`). Respuesta 200: `active`, identificadores
eco y `session_expires_at` UTC (null si inactivo). `active=true` exige coincidencia
cuenta/sesión, cuenta activa, sesión no revocada y no vencida, además de
elegibilidad por método: `password` y `legacy` requieren contacto verificado y
credencial local; `google` requiere su identidad Google asociada a esa cuenta.
Una identidad externa de la cuenta no habilita sus sesiones locales pendientes.
Sesión inexistente, cuenta distinta, estado bloqueado o revocado: `active=false`
sin revelar motivo ni datos de la cuenta real. No cachear negativos en esta etapa.
El lote devuelve exactamente un resultado por par, mismo orden; una evaluación
no obtenible hace fallar el lote completo con 503, nunca inventa `active=false`.
Es una lectura autoritativa al evaluar, no una reserva ni bloqueo de autorización.

Ejemplo individual (`POST /internal/v1/sessions/check`), UUID ficticios:

```json
{
"sid":"22222222-2222-4222-8222-222222222222",
"account_id":"11111111-1111-4111-8111-111111111111"
}
```

```json
{
"sid":"22222222-2222-4222-8222-222222222222",
"account_id":"11111111-1111-4111-8111-111111111111",
"active":true,
"session_expires_at":"2026-11-10T12:00:00Z"
}
```

El lote (`POST /internal/v1/sessions/check-batch`) envuelve los mismos pares en
`{"sessions":[...]}` y responde `{"results":[...]}`. No acepta tokens de usuario
en el cuerpo ni devuelve credenciales.

Errores `application/problem+json`: 400 `/problems/malformed-request`,
401 `/problems/invalid-service-credentials`, 403 `/problems/service-forbidden`,
413 `/problems/request-too-large`, 429 `/problems/service-rate-limited`,
503 `/problems/session-check-unavailable`. No incluir secretos ni internals.
Respuestas HTTP internas `Cache-Control: no-store`; la microcaché es explícita
en el consumidor, no una caché HTTP intermediaria.

## Propuestas pendientes para consumidores

Las reglas siguientes describen el diseño previsto de microcaché, difusión y
WebSocket en Core/Files. No acreditan una implementación de esos consumidores.
El circuit breaker propuesto abre cinco segundos tras cinco fallos consecutivos
de transporte, timeout o 5xx; permite una prueba en half-open y vuelve a abrir
ante fallo. Los errores 401/403 requieren revisión de credenciales, y 429 respeta
Retry-After; ninguno equivale a revocación de usuario.

### Semántica temporal y carreras


Al iniciar consulta, capturar `t0` monotónico y generación local por sid. Al
recibir positivo a `t1`, deadline monotónico como máximo `t0 + 10 s`, acotado
también al vencimiento absoluto devuelto. Convertir el vencimiento UTC con reloj
sincronizado y margen conservador para desfase (propuesta 30 s); si no puede
garantizarse el límite absoluto, rechazar. Si `t1 >= deadline`, descartar resultado.
Consultar una entrada no cambia deadline. Cada JWT conserva su propio `exp`:
una entrada nunca autoriza un JWT expirado, de otra cuenta o inválido.
En lote t0 es el inicio del lote para todos los pares, no el final de cada consulta.

Revocación conocida incrementa generación e invalida positivo y sockets en una
sección atómica por sid. La respuesta solo se instala/usa si la generación sigue
igual y no existe tombstone de revocación. Conservar tombstone al menos hasta
expiración de sesión más margen; si evento llegó antes de iniciar consulta,
tampoco aceptar positivo posterior. No reutilizar sid para nuevas sesiones.
Al reiniciar consumidor se descartan positivos; recuperar consumo antes de
restaurar sockets. Una partición PubSub exige revalidación y estado cerrado al
vencer deadline; la difusión no es garantía instantánea durante particiones.

Ejemplo: consulta tarda 0.8 s; quedan como máximo 9.2 s, no diez segundos nuevos.
Evento durante consulta impide instalar respuesta favorable tardía. Si hay un
timeout y quedan dos segundos del positivo anterior puede usarse durante esos
dos segundos; después, nueva comprobación o 503. Ni fallo ni 503 borra refresh.
Negativo autoritativo se traduce a 401 de sesión no utilizable en API protegida.

WebSocket mantiene deadlines independientes JWT/comprobación, compara ambos
antes de cada acción y entrega; programa renovación de comprobación antes de
vencer (propuesta 5 s con jitter). Sin positivo vigente se suspenden entregas y
acciones; notificar indisponibilidad por protocolo y cerrar si no se recupera.
Expiración JWT exige reautenticación por protocolo aún por definir o reconexión;
no se amplía por tener socket abierto. Revocación conocida cierra inmediatamente.
No se cancela retroactivamente trabajo autorizado. Files debe definir límites
para transferencias largas antes de implementar, sin prometer corte inmediato.
