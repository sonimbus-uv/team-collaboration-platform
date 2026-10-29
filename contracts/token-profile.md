# Perfil de tokens y JWKS v1

Diseño de [D-16 / ADR-007](../docs/decisions/ADR-007-auth-sessions.md), Refs #35.
Estado de la tercera entrega: emisor/JWKS y sesiones públicas implementados;
login deshabilitado por defecto por #17. Integración de consumidores pendiente. Ver
[operación y validación](../services/auth/TOKENS.md). Afecta Auth, Core, Files
y ambos clientes. Los identificadores y tiempos de ejemplo son ficticios;
no hay firmas ni secretos utilizables.

## Credenciales separadas

| Credencial | Emisor / uso | Persistencia |
|---|---|---|
| Access JWT | Auth / APIs protegidas y conexión WS | Memoria del cliente |
| Refresh opaco | Auth / refresh y logout de su sesión | Llavero SO; solo hash en Auth |
| Token de verificación | Auth / confirmar correo, propósito único | Hash en `one_time_tokens`; no es refresh |
| Bearer interno de servicio | Operación / consultar sesiones desde Core o Files | Solo backend, permisos de consulta |
| Credencial AMQP | Operación / publicar o consumir eventos | Solo backend; independiente de HTTP |
| JWT LiveKit | Core VoiceService / medios (D-15) | Perfil distinto, nunca aceptado como access |

Propuesta técnica: refresh de 32 bytes CSPRNG codificados base64url sin padding
(43 caracteres), digest SHA-256 de sus bytes; no JWT. Hashes consumidos se
conservan hasta expirar la sesión. No registrar cuerpos de login/refresh/logout,
Authorization, claves ni tokens en trazas o excepciones. El digest no se expone.

## JWT de acceso

JWS compacto; encabezado obligatorio `alg: EdDSA`, `typ: at+jwt`, `kid` único
no reutilizado. Clave exclusivamente Ed25519. Rechazar `none`, otros algoritmos,
curvas, encabezados críticos no soportados y selección de clave por `jku`/`x5u`
del token. El consumidor solo usa su JWKS configurado y vinculado al issuer.

| Claim requerido | Tipo / validación |
|---|---|
| `iss` | URI exacta configurada por entorno; sin descubrimiento desde el token |
| `aud` | Array exacto de destinatarios permitidos; propuesta `["sonimbus-core", "sonimbus-files", "sonimbus-auth"]`; cada receptor exige su audiencia |
| `sub` | UUID de `auth.accounts.id`, identidad de cuenta |
| `sid` | UUID de sesión, vinculado a `sub` mediante comprobación Auth |
| `iat` | NumericDate entero UTC, instante de emisión |
| `exp` | NumericDate entero UTC, `iat < exp <= iat + 900`, nunca posterior a `session_expires_at` |
| `jti` | UUID único por emisión; no sustituye a `sid` ni a comprobación |

No incluir email, contraseña, refresh, permisos de grupos ni perfil personal.
Propuesta: tolerancia de reloj de 30 s solo para `iat` futuro; `exp` y el límite
absoluto son deadlines estrictos sin gracia de autorización. Los hosts deben
sincronizar reloj y detectar saltos. Rechazar tipos erróneos, campos duplicados,
firma inválida, issuer/audiencia incorrectos y `kid` desconocido no resoluble.
La cuenta y sesión deben ser autorizables además de la firma correcta.

```json
{"alg":"EdDSA","typ":"at+jwt","kid":"auth-example-2026-10"}
```

```json
{"iss":"https://auth.example.invalid","aud":["sonimbus-core","sonimbus-files","sonimbus-auth"],"sub":"11111111-1111-4111-8111-111111111111","sid":"22222222-2222-4222-8222-222222222222","iat":1792627200,"exp":1792628100,"jti":"33333333-3333-4333-8333-333333333333"}
```

Los clientes pueden leer tiempos para programar refresh, pero esos datos no
constituyen autorización. Cada servicio verifica JWT aunque su sid esté cacheado.

## JWKS y ciclo de claves

`GET /.well-known/jwks.json`, público por gateway, JSON con `keys` no vacío.
Cada clave contiene `kty: OKP`, `crv: Ed25519`, `use: sig`, `alg: EdDSA`, `kid`
y `x` (32 bytes base64url sin padding). Nunca `d`. `kid` único dentro del conjunto.
Ejemplo estructural deliberadamente sin material criptográfico:

```json
{"keys":[{"kty":"OKP","crv":"Ed25519","use":"sig","alg":"EdDSA","kid":"auth-example-2026-10","x":"<public-key-32-bytes-base64url>"}]}
```

`Cache-Control: public, max-age=300, must-revalidate`; caché local máxima cinco
minutos, no deslizante. No usar JWKS vencido si falla su actualización: negar
temporalmente (HTTP 503 cuando corresponda). Clave desconocida: una actualización
acotada y coalescida, con rate limit para impedir descargas por cada kid atacante;
si actualización exitosa sigue sin kid, 401. Fallo de red no demuestra JWT inválido.
Configuración de issuer/URL, certificados y caché son independientes de sesiones.

Privada en archivo protegido, montado solo en Auth, lectura mínima de su usuario,
fuera del repositorio. Rotar cada 90 días: publicar primero la nueva pública,
esperar al menos los cinco minutos de propagación más margen, empezar a firmar
con nuevo kid y detener firma anterior. Retirar pública anterior solo después de
la última expiración emitida con ella más tolerancia de reloj y margen operativo.
Propuesta de margen: cinco minutos; registrar última emisión y expiración reales.

Pendiente política de compromiso: retirada de emergencia, invalidación de caches,
revocación de sesiones afectadas, notificación y recuperación. Rotación programada
no resuelve compromiso; no prometer invalidación instantánea de claves cacheadas.
Si no puede publicarse la nueva clave, no cambiar a ella. Si no puede leerse la
privada o firmar, no emitir tokens; 503 y alerta sin datos secretos. Verificar
operativamente propagación y rollback antes de automatizar. Valores propuestos
(audiencias, tolerancia, margen y formato refresh) requieren revisión del equipo.
