# ADR-007: Autenticación y sesiones con verificación distribuida

## Estado

Decisiones confirmadas por el solicitante para la primera entrega de #35;
pendiente de revisión del equipo y aprobación del ADR. Diseño, sin implementación.
Registro D-16; desarrolla D-02, D-04, D-07, D-12 y D-13.

## Fecha

2026-10-22.

## Contexto

Inspección sobre `07ec9bf`, rama de partida `fix/auth-rate-limiting`, árbol limpio.
La rama de documentación parte de ese commit; un futuro PR debe revisar también
su base para no incorporar inadvertidamente commits previos de rate limiting.
No se encontraron AGENTS.md en el repositorio ni sus directorios ascendentes.

La [migración actual](../../services/auth/migrations/0001_initial_auth_schema.sql)
contiene cuentas, sesiones, refresh y `auth.open_session`, pero no una outbox.
La función serializa logins por cuenta/tipo y reemplaza sesiones; acepta un TTL
arbitrario: todavía no impone 20 días. Las tablas permiten rotación, pero no
implementan por sí mismas el protocolo de refresh ni detección de reutilización.
Las rutas Rust solo ofrecen registro, verificación, reenvío y salud. No hay emisión
JWT, login, refresh, logout ni JWKS. Core y Files no tienen esta integración.

El [gateway](../../gateway/nginx/routes/auth.conf) publica `/auth/...`, sin
`/api/v1`, JWKS ni API interna; su configuración actual escucha HTTP. HTTPS es
un requisito de la implementación futura, no una garantía existente.
El issue [#35](https://github.com/sonimbus-uv/team-collaboration-platform/issues/35)
planteaba Redis para revocación; se elige PostgreSQL autoritativo y consulta HTTP.
[#14](https://github.com/sonimbus-uv/team-collaboration-platform/issues/14) aún
describe diferencias históricas: ya existe la página HTML de verificación.
No se decide aquí el contrato completo de registro/verificación.

La [propuesta de escritorio](../../clients/desktop/docs/issues-auth-escritorio.md)
de refresh deslizante de 30 días y fallback automático a memoria queda sustituida
por este diseño. Su texto se conserva como propuesta histórica, no normativa.

## Decisión

- JWT Ed25519, verificado localmente por cada servicio. Access máximo 15 minutos,
  acotado al vencimiento de sesión. Sesión absoluta de 20 días desde login,
  sin extensión por refresh. Una sesión desktop y una mobile por cuenta;
  el último login del mismo tipo reemplaza la anterior.
- Refresh opaco aleatorio, solo hash en Auth, rotación transaccional en cada uso.
  Reutilización de uno consumido revoca toda la sesión. El cliente serializa
  renovaciones; resultado incierto exige login, sin reintento del refresh usado.
- Access en memoria; refresh en llavero del SO. Cerrar la app conserva sesión;
  abrirla restaura por refresh. Logout revoca. Llavero no disponible: pendiente,
  sin alternativa en claro ni fallback acordado.
- Core y Files consultan Auth por HTTPS + JSON sobre red privada directamente,
  verificando certificado. No leen tablas de Auth. Credenciales HTTP opacas
  distintas por servicio y permisos mínimos; sin mTLS ni OAuth Client Credentials.
  Permanecen en backend, fuera de repositorio, URLs y logs.
- Microcaché positiva por `(sid, account_id)` de hasta 10 segundos, sin renovar
  TTL al leerla. Validación JWT individual en cada solicitud. Vencidos no se
  reutilizan ante errores. Los eventos invalidan anticipadamente; una respuesta
  tardía no puede resucitar una revocación conocida. El presupuesto incluye la
  latencia; ver [semántica temporal](../../contracts/session-checks.md).
- Sin Auth, solo se continúa con JWT válido y comprobación positiva vigente.
  Si se requiere una nueva y falla: 503, no revocación, sin borrar refresh.
  No se promete cancelar retroactivamente operaciones ya autorizadas.
- RabbitMQ acelera invalidación y desconexión; ausencia de evento no acredita
  actividad. JSON por `sonimbus.events`; revocación y outbox en misma transacción,
  publicación confirmada, reintentos, consumidores idempotentes. AMQP sobre TLS,
  credenciales distintas de HTTP. Core consume en cola durable de servicio y
  difunde con Phoenix.PubSub a nodos. Consumidores competidores no son broadcast.
  Files debe definir difusión entre instancias y tratamiento de transferencias.
- WebSocket comprueba al conectar, antes de acciones protegidas y periódicamente.
  Entregas usan estado del socket con deadlines JWT y sesión explícitos, nunca
  HTTP por entrega. Cada acción/entrega compara tiempo actual con ambos deadlines,
  aunque el temporizador se retrase. Revocación invalida y cierra. Errores en una
  conexión establecida se expresan por su protocolo, no como HTTP 503.
- Privada Ed25519 en archivo protegido accesible solo por Auth. JWKS público
  `/.well-known/jwks.json` por gateway; caché cinco minutos independiente de sesión.
  Rotación cada 90 días y convivencia de públicas hasta vencer tokens anteriores
  más tolerancia de reloj y margen operativo. Compromiso y fallos de actualización
  requieren política separada; ver [perfil](../../contracts/token-profile.md).

## Alternativas consideradas

PASETO no se adopta: D-07 confirma JWT Ed25519. Validar solo `exp` no permite la
ventana de revocación acordada. Acceder a Redis o PostgreSQL de Auth desde otros
servicios viola D-13. Consulta sin caché aumenta dependencia y carga; se acepta
hasta 10 segundos de desactualización para nuevas decisiones. Solo RabbitMQ no
demuestra sesión activa ante pérdidas o retrasos. mTLS y OAuth añaden gestión
fuera del alcance de esta etapa. Refresh deslizante y reintentos transparentes
tras resultado incierto contradicen el límite absoluto y detección de reutilización.

## Consecuencias y garantías reales

Una revocación puede tardar hasta el límite restante de comprobación en afectar
nuevas autorizaciones; RabbitMQ puede reducirlo, sin SLA de entrega instantánea.
La disponibilidad de Auth condiciona nuevas comprobaciones. La outbox requiere
una migración posterior y adaptar también las revocaciones dentro de
`auth.open_session`; publicar solo desde el handler perdería eventos.
Un refresh perdido tras commit obliga a login; no hay ventana de gracia.
Los hashes consumidos deben conservarse durante la vida de la sesión para
detectar replay; limpieza y retención son trabajo posterior.

[#17](https://github.com/sonimbus-uv/team-collaboration-platform/issues/17) es una
condición previa a habilitar login: reenviar/verificar el correo de una cuenta
creada por un atacante no reemplaza su contraseña. El dueño podría verificar
credenciales que controla el atacante. Se requiere recuperación segura y pruebas
de carreras coherentes con enumeración; este ADR no elige ni implementa esa solución.

El [plan](../auth-sessions-implementation.md) separa diseño e implementación.
CA-01 de #35 espera revisión/aprobación; CA-02 aporta contratos por revisar;
CA-03 (PoC Rust–Elixir) permanece pendiente. No se declara interoperabilidad.
