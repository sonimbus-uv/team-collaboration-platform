# Validación de integración de Auth — 2026-11-14

Validación local sobre `review/auth-final-validation`, basada en `1d58fa9`, con
las correcciones de formato `65a2491` y de documentación `a3442c2`.
No constituye resultado de GitHub Actions ni aprobación de despliegue.

## Entorno y aislamiento

VM Ubuntu de Vagrant, Rust, OpenSSL y Nginx. Contenedores desechables
`postgres:17-alpine`, `redis:8-alpine` y `axllent/mailpit:v1.29.7`.
SQLx creó bases independientes por prueba. Las colecciones usaron una base
desechable, claves y credenciales generadas para esa ejecución, Mailpit aislado y
un proceso Auth con login y contraseñas habilitados. Redis se vació exclusivamente
en el contenedor desechable antes de cada colección, conservando las cuotas del
servicio. Los contenedores, tokens y material privado se retiraron al terminar.

## Resultados

| Comprobación | Resultado |
|---|---|
| `cargo fmt --check` | Correcto tras corregir formato en seis archivos |
| `cargo check --all-targets --locked` | Correcto |
| `cargo clippy --all-targets --locked -- -D warnings` | Correcto |
| `cargo test --locked -- --include-ignored` | 239 aprobadas, 0 fallidas, 0 ignoradas |
| `cargo build --locked` | Correcto; binario utilizado para Postman |
| `bash -n scripts/auth/dev/start-auth-dev.sh` | Correcto |
| `test-auth-dev-setup.py` | 1 prueba aprobada |
| `test-gateway-tls-setup.py` | 2 pruebas aprobadas |
| `test-auth-jwks-gateway.py --nginx /usr/sbin/nginx` | Correcto |
| La misma prueba con `--tls` | Correcto |
| `test-auth-registration-code.py --confirm-isolated` | Correcto contra Auth y Mailpit desechables |
| Newman: `auth-local.postman_collection.json` | 32 aserciones, 0 fallos |
| Newman: `auth-internal.postman_collection.json` | 62 aserciones, 0 fallos |
| Newman: `auth-passwords.postman_collection.json` | 39 aserciones, 0 fallos |
| `actionlint` sobre `.github/workflows/auth.yml` | Correcto |
| Archivos OpenAPI de Auth | JSON válido y 194 referencias resueltas |
| Enlaces locales de las guías vigentes de Auth, contratos y gateway | Destinos existentes |

Newman se ejecutó con `postman/newman:alpine`, una iteración, pausa de 1000 ms,
variables de entorno privadas y confianza en la CA desechable mediante
`NODE_EXTRA_CA_CERTS`. No se deshabilitó la verificación TLS. Las colecciones
accedieron directamente al proceso Auth; el gateway se comprobó por separado
contra un backend desechable mediante sus pruebas HTTP y HTTPS.

## Límites

Google se validó con el proveedor simulado y las pruebas OIDC de Rust, sin
consentimiento real. No se ejecutaron el smoke interactivo de Google en Windows,
la instalación de la CA en Windows ni pruebas de clientes nativos.
La revisión de documentación comprobó destinos de archivos; no validó
automáticamente todos los fragmentos de encabezado de Markdown.
Postman se ejecutó localmente: el workflow actual ejecuta Rust y gateway, sin un
job Newman. El resultado remoto debe comprobarse en el PR.
