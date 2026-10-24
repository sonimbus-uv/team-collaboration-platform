# Comprobación interna de sesiones v1

Refs #35. Diseño pendiente de implementación, ligado al
[OpenAPI](openapi/auth-internal.yaml) y [ADR](../docs/decisions/ADR-007-auth-sessions.md).
Auth es autoridad en PostgreSQL. Solo Core y Files, por HTTPS + JSON en red
privada sin Nginx público; verificación de hostname y cadena del certificado.
Nunca consultar almacenamiento interno de Auth desde consumidores.

## Autenticación y límites

Propuesta: `Authorization: Bearer <service-credential>`, secreto aleatorio opaco
por servicio, asociado en Auth a identidad `core` o `files` y permiso
`sessions:check`. No es JWT de usuario, refresh ni credencial de RabbitMQ.
Rotación con solapamiento acotado y revocación independiente por identidad;
duración exacta y distribución de secretos pendientes. No aceptar identidad
declarada en el cuerpo ni guardar secreto en URLs, repositorio o logs.

Valores propuestos para revisar con mediciones: timeout total 1 s (conexión
300 ms incluida), sin reintento automático; lote 1–100 pares distintos,
64 KiB por petición; 100 consultas/s y 20 en vuelo por identidad, agregadas
entre réplicas de Auth (cada par cuenta como consulta). 429 con Retry-After
en segundos; 413 para tamaño y 400 para lote inválido/duplicado. Control de
concurrencia y cuotas antes de acceso DB. Protección sin autenticar: 10 req/s
por origen privado, cuerpo acotado y presupuesto global de 50 conexiones en
autenticación; no sustituye verificar credencial. Cuotas independientes del
rate limiting público por IP. Elegidas para acotar carga, no como capacidad demostrada.

Circuit breaker propuesto por consumidor/destino: cinco fallos de transporte,
timeouts o 5xx consecutivos abren 5 s; half-open permite una prueba; éxito cierra,
fallo reabre. 401/403 internos indican configuración/credencial y generan alerta;
429 respeta Retry-After. Ninguno equivale a revocación de usuario. Circuito
abierto no invalida un positivo aún vigente, ni permite usar uno vencido.

## Consulta y errores

POST individual y lote reciben pares `sid`, `account_id` UUID tomados del JWT
ya validado (`sub` → `account_id`). Respuesta 200: `active`, identificadores
eco y `session_expires_at` UTC (null si inactivo). `active=true` exige coincidencia
cuenta/sesión, cuenta activa y verificada, sesión no revocada y no vencida.
Sesión inexistente, cuenta distinta, estado bloqueado o revocado: `active=false`
sin revelar motivo ni datos de la cuenta real. No cachear negativos en esta etapa.
El lote devuelve exactamente un resultado por par, mismo orden; una evaluación
no obtenible hace fallar el lote completo con 503, nunca inventa `active=false`.
Es una lectura autoritativa al evaluar, no una reserva ni bloqueo de autorización.

Ejemplo individual (`POST /internal/v1/sessions/check`), UUID ficticios:

```json
{"sid":"22222222-2222-4222-8222-222222222222","account_id":"11111111-1111-4111-8111-111111111111"}
```

```json
{"sid":"22222222-2222-4222-8222-222222222222","account_id":"11111111-1111-4111-8111-111111111111","active":true,"session_expires_at":"2026-11-10T12:00:00Z"}
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

## Semántica temporal y carreras

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
