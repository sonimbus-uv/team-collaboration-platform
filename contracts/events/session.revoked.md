# session.revoked v1

Refs #35. Productor Auth; consumidores Core y Files. Diseño por implementar.
[JSON Schema](session.revoked.v1.schema.json), [ADR](../../docs/decisions/ADR-007-auth-sessions.md).
Envelope versionado según el catálogo, exchange topic `sonimbus.events`, routing
key `session.revoked`. `event_id` UUID estable para todos los reintentos;
`occurred_at` y `revoked_at` UTC del cambio confirmado, no del reintento.

```json
{
  "type": "session.revoked",
  "version": 1,
  "event_id": "44444444-4444-4444-8444-444444444444",
  "occurred_at": "2026-10-22T12:00:00Z",
  "payload": {
    "sid": "22222222-2222-4222-8222-222222222222",
    "account_id": "11111111-1111-4111-8111-111111111111",
    "client_type": "desktop",
    "reason": "logout",
    "revoked_at": "2026-10-22T12:00:00Z",
    "session_expires_at": "2026-11-10T12:00:00Z"
  }
}
```

Motivos exactamente compatibles con `sessions_revoke_reason_chk`: `logout`,
`superseded`, `expired`, `refresh_reuse`, `password_changed`, `admin`. Bloqueo,
suspensión o baja revocan sesiones como `admin`; no introducir nuevos motivos sin
evolucionar esquema/contrato. Expiración se rechaza por reloj aunque el job que
la marque y publique `expired` no haya corrido. No datos personales, tokens ni hashes.

## Producción y recuperación

Cada transición a revocada y su evento pendiente se guardan en una transacción
PostgreSQL. Debe abarcar logout, replay, reemplazo en `auth.open_session`, cambios
de contraseña, administración y limpieza. La outbox aún no existe: migración y
adaptación de la función SQL son dependencias futuras. No publicar antes de commit.

Worker publica mensajes persistentes con publisher confirms y detección de
mensajes no enrutados; marca enviado solo tras confirmación y enrutamiento.
Fallo entre confirm y marcado puede duplicar: entrega al menos una vez, sin
orden global ni exactly-once. Reintentos propuestos con backoff 1–60 s y jitter,
sin descartar silenciosamente pendientes; alertar por edad/profundidad. RabbitMQ
indisponible no revierte revocación; consulta sigue autoritativa. AMQP con TLS
y verificación de servidor, usuarios/permisos mínimos separados de HTTP.

## Consumo, difusión y sockets

Colas durables de servicio enlazadas al routing key y DLQ. Validar versión,
esquema y límites de tamaño (propuesta 8 KiB) antes de aplicar. Ack manual tras
invalidación y entrega a un mecanismo recuperable de difusión local del servicio.
Versiones desconocidas/mensajes inválidos a DLQ con alerta, sin bucle infinito.
Errores transitorios reintentan con backoff acotado y recuperación de conexión.

Idempotencia por event_id y, además, por sid revocado: duplicados o eventos fuera
de orden nunca reactivan sesión. Instalar tombstone, invalidar microcaché y estado
de sockets, cerrar conexiones. Mantener barrera frente a consultas HTTP en vuelo
según [comprobación](../session-checks.md). Dedupe y tombstones deben cubrir al
menos vida restante de sesión más margen operativo; no depender solo de timestamp.

Core propone cola durable `core.session-revoked.v1`, trabajadores recuperables y
Phoenix.PubSub para propagar invalidación a todos los nodos, sus caches y sockets.
Una cola con consumidores competidores entrega a uno de ellos, no a cada nodo.
PubSub no es durable: caída entre ack y difusión o partición puede perder la
aceleración; revalidación por deadlines de hasta 10 s conserva el límite de
autorización. Recuperación exige descartar positivos previos y recomprobar.
El mecanismo recuperable, orden ack/difusión y pruebas multinodo quedan por implementar.

Files: integración explícitamente pendiente. Antes de habilitarla, elegir difusión
entre todas sus instancias (por ejemplo consumidor de servicio más PubSub si se
adopta clustering, o colas durables por instancia con gestión de ciclo de vida),
definir qué ocurre con transferencias en curso, partes, URLs firmadas y reanudación.
No asumir que cola compartida invalida caches de cada instancia ni prometer
cancelación retroactiva. Ausencia de evento nunca demuestra sesión activa.
