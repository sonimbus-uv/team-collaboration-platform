# Firma de access tokens y JWKS

Segunda entrega de #35; su estado histórico se describe debajo. La tercera entrega
integra el emisor con [sesiones públicas](SESSIONS.md), con login deshabilitado
por #17. Sigue [ADR-007 / D-16](../../docs/decisions/ADR-007-auth-sessions.md)
y el [perfil](../../contracts/token-profile.md). Implementa carga de claves,
emisión como componente Rust y publicación pública. Actualmente también existen
login, refresh, logout y persistencia de outbox, descritos en SESSIONS.md. No hay
consulta privada de sesiones, publicación RabbitMQ ni PoC Rust–Elixir completada.

## Dependencias y separación

`security/tokens/` se exporta desde la biblioteca `auth_service::tokens`:
`config` valida configuración; `keys` carga y valida el snapshot privado/público;
`claims` valida contexto y calcula vencimientos; `issuer` emite JWT con una
biblioteca JWT, sin construir JWS manualmente. `routes/jwks.rs` publica el snapshot.

Se fija `jsonwebtoken` 10.4.0 con `default-features=false` y `rust_crypto`;
admite EdDSA y claves DER PKCS#8. Se fija `ed25519-dalek` 2.2.0 con `pem` para
validar Ed25519 PKCS#8 y derivar la pública que se contrasta con JWKS. No se
habilitan claves RSA, EC u otros algoritmos en el componente emisor, aunque el
backend de la biblioteca los incluya transitivamente. `base64` codifica JWK,
`zeroize` protege el buffer PEM y `uuid` genera jti v4. `libc` comprueba propietario
y apertura sin symlinks en Unix. Cargo.lock incorpora estas dependencias sin
actualizaciones ajenas de paquetes existentes.

Mínimos declarados por las nuevas bibliotecas: Rust 1.88 para jsonwebtoken 10.4
y 1.81 para dalek 2.2. El repositorio no tenía MSRV global declarado y CI utiliza
stable: no se cambia la edición 2021 ni se promete que todo el lock compile con
1.88. La validación local utilizó Rust 1.99.1 GNU en Windows, instalado únicamente
en un directorio temporal sin modificar PATH persistente.

Fuentes oficiales consultadas:

- [jsonwebtoken: versión mínima y backend](https://docs.rs/crate/jsonwebtoken/10.4.0/source/Cargo.toml).
- [EncodingKey y formatos](https://docs.rs/jsonwebtoken/10.4.0/jsonwebtoken/struct.EncodingKey.html).
- [dalek: PKCS#8 PEM y Ed25519](https://docs.rs/ed25519-dalek/2.2.0/ed25519_dalek/).
- [dalek: versión mínima](https://docs.rs/crate/ed25519-dalek/2.2.0/source/Cargo.toml).

## Configuración de arranque

Además de las variables existentes de PostgreSQL, Redis y correo, son obligatorias:

| Variable | Significado |
|---|---|
| `AUTH_JWT_PRIVATE_KEY_PATH` | Archivo privado Ed25519 PKCS#8 PEM sin contraseña |
| `AUTH_JWT_JWKS_PATH` | Archivo JSON público con activa y anteriores retenidas |
| `AUTH_JWT_ACTIVE_KID` | Identificador de la clave activa presente en JWKS |
| `AUTH_JWT_ISSUER` | Issuer exacto HTTPS; sin credenciales, query ni fragmento |
| `AUTH_JWT_AUDIENCES` | Identificadores distintos separados por comas, sin espacios |

Los valores de `.env.example` son referencias sin secretos y sus archivos no
existen automáticamente. El operador debe aprovisionarlos explícitamente fuera
del repositorio. Auth falla al arrancar antes de abrir PostgreSQL si falta o es
inválida una clave; no genera fallback. No se crean claves de producción en pruebas.

Archivos regulares, no symlinks, máximo 64 KiB; JWKS de 1 a 16 públicas Ed25519.
Cada JWK contiene exactamente `kty`, `crv`, `use`, `alg`, `kid`, `x`, conforme al
contrato; se rechazan `d`, otros campos, algoritmos/curvas incompatibles, claves
débiles, codificación no canónica y kids duplicados. `kid` admite 1–128 caracteres
ASCII alfanuméricos, punto, guion y guion bajo. Son límites locales de protección
del cargador, adicionales a la estructura del contrato.

En Unix la privada debe pertenecer al usuario efectivo de Auth y no conceder
permisos a grupo u otros (`0400` o `0600` recomendados). Proteger también los
directorios y usar un usuario exclusivo y montaje solo para Auth. La pública
debe poder leerse por Auth y ser modificable únicamente por operación confiable.
En Windows esos permisos POSIX no existen: el operador debe establecer una ACL
restringida a la identidad de Auth y administradores; el cargador no verifica
ACL Windows. No desplegar con ACL heredada de acceso general.

## Emisión como componente

`AccessTokenIssuer::issue(&SessionContext)` recibe UUID de cuenta/sesión,
login_at y expires_at como segundos Unix. Su llamador debe haber
autenticado la cuenta y comprobado pertenencia, actividad y no revocación de
sesión. El componente no consulta ni inventa sesiones y no prueba esas condiciones.

Rechaza UUID nil, login cero/futuro, sesión vencida, expiración inconsistente,
duración de sesión superior a veinte días y desbordamientos. Usa reloj del
servidor; `exp = min(iat + 900, expires_at)`. Los timestamps de sesión que
provengan de precisión subsegundo deben truncarse hacia abajo por el llamador.
No hay gracia que prolongue exp. JWT incluye `iss`, aud array, `sub`, `sid`,
`iat`, `exp`, `jti`; encabezado EdDSA, at+jwt y kid. Issuer/audiencias solo vienen
de configuración confiable. El String retornado es sensible: el llamador
debe evitar logs, Debug de respuestas y persistencia innecesaria.

## Endpoint y gateway

`GET /.well-known/jwks.json` y HEAD público, sin sesión ni presupuestos de
registro/verificación/reenvío/destinatario. JSON exclusivamente público y snapshot
inmutable validado al arrancar, ordenado por kid. No combina archivos durante
una solicitud. `Cache-Control: public, max-age=300, must-revalidate` y `nosniff`.
La ruta exacta Nginx conserva la respuesta de Auth y se registra como operación
`jwks`, sin alterar firewall, proxy confiable o límites de otras rutas.

El contrato aprobado fija Cache-Control, pero no ETag, Last-Modified ni 304:
no se introducen validadores HTTP no acordados. La revalidación actual obtiene
200 completo; cualquier incorporación de validadores debe actualizar el contrato.
No confundir cinco minutos JWKS con la futura microcaché de sesión de diez segundos.

Con archivos y entorno aprovisionados y servicios existentes disponibles:

```sh
cd services/auth
cargo run --locked
# En otra terminal, con la URL del gateway de desarrollo existente:
curl -i http://localhost:8080/.well-known/jwks.json
curl -I http://localhost:8080/.well-known/jwks.json
```

La configuración Nginx actual del repositorio utiliza HTTP de desarrollo;
producción requiere HTTPS en el frente público según ADR. Esta entrega no
configura certificados ni habilita acceso directo público a Auth. Las pruebas
del router no requieren levantar el proceso Auth ni sus servicios dependientes.

## Rotación operativa cada 90 días

1. El operador aprovisiona una nueva privada fuera del repositorio, con kid nunca
   reutilizado, y prepara un JWKS con activa anterior y nueva pública.
2. Publicar ese JWKS en todas las réplicas, manteniendo la privada/kid anteriores
   activos, mediante reinicio controlado con snapshot validado. Verificar que
   todas publican exactamente el conjunto esperado.
3. Esperar al menos cinco minutos más el margen operativo acordado (el perfil
   propone cinco minutos adicionales). No activar si falla publicación o propagación.
4. Cambiar privada y kid activos y reiniciar de forma controlada; conservar ambas
   públicas. Registrar última emisión/expiración con la clave anterior entre todas
   las réplicas, incluyendo las que no hayan completado el cambio.
5. Retirar la pública anterior solo después de vencer el último token firmado
   con ella, más tolerancia de reloj y margen operativo. No calcular ese instante
   desde el inicio de la rotación si hubo réplicas aún firmando con la anterior.

No hay hot reload ni rotación automática. Un reinicio con archivos inconsistentes
falla; no sustituye silenciosamente claves. Respaldos/rollback deben preservar
publicación y retención; no reactivar una clave retirada o comprometida sin una
política explícita. El compromiso de claves continúa pendiente de política propia;
la caché pública impide prometer retirada instantánea.

## Pruebas y limitaciones

```sh
cargo fmt --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
# Sin PostgreSQL ni Redis, comprobar solo la biblioteca:
cargo test --lib --locked
cargo test --locked jwks_route_is_public
# Solo con PostgreSQL de pruebas y Redis configurados:
cargo test --locked -- --include-ignored
```

El Clippy estricto se aplica a todos los targets. Las pruebas crean
material fijo desechable solo en temporales y borran únicamente sus propios archivos.
En CI Ubuntu se ejecutan además las comprobaciones Unix de permisos y symlinks.
La prueba de router completo con DB/Redis inaccesibles no realiza consultas a ellos.

Desde la raíz, con Nginx instalado:

```sh
python3 scripts/test-auth-jwks-gateway.py --nginx /usr/sbin/nginx
```

Ejecuta Nginx real con copias temporales de su configuración y backend HTTP stub,
puertos loopback efímeros, GET/HEAD, caché, independencia de presupuestos y 404.
No demuestra integración de Auth con PostgreSQL/Redis ni interoperabilidad Elixir.
El CI existente mantiene sus pruebas completas con servicios de prueba y añade
este chequeo de gateway. No se declara CI remoto exitoso antes de ejecutarlo.

## Registro histórico de la segunda entrega

Los resultados y referencias de este apartado corresponden a esa entrega,
no al inventario actual ni al estado remoto de sus PR. La validación vigente
se ejecuta con los comandos anteriores y se registra en [VALIDATION.md](VALIDATION.md).

Resultado local de esa entrega: formato y `cargo check --all-targets --locked`
correctos; Clippy estricto de biblioteca y Clippy de todos los targets correctos
(con las dos advertencias previas del binario). Ocho pruebas de biblioteca,
42 pruebas del binario y la prueba adicional de router completo pasan: 51 en
total. Tres pruebas Redis permanecen ignoradas y siete pruebas SQLx dependientes
de PostgreSQL no se ejecutaron. Permisos/symlinks Unix esperan ejecución en CI
Linux. La prueba de gateway pasa con Nginx Windows 1.30.6 oficial; ni ese resultado
ni las pruebas Rust acreditan interoperabilidad Elixir o CI remoto exitoso.

Base: rama dependiente de `70cfb9b`, PR #44 todavía abierto al comenzar. Un PR de
esta entrega debe comparar contra la rama documental hasta integrarla; después
revisar su base sin reescribir historia ni duplicar cambios. Próxima entrega:
resolver #17 y enumeración antes de habilitar login; luego sesiones transaccionales.
