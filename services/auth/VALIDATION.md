# Revisión de Auth y validación de la reorganización

Nota 2026-10-28: este informe conserva la validación histórica. El flujo de
registro con enlaces fue sustituido por [códigos e intentos](REGISTRATION.md);
ver [validación de #17](REGISTRATION_VALIDATION.md) para el estado posterior.

Fecha: 2026-10-24. Rama: `fix/auth-registration-e2e`.

## Alcance revisado

Se contrastaron README.md, REGISTRATION.md, RATE_LIMITING.md, TOKENS.md y
SESSIONS.md con rutas, handlers, DTO, configuración, servicios, repositorios,
proveedores de correo, protección, emisor JWT y migraciones. También se revisaron
las configuraciones de Nginx, aprovisionamiento, firewall y workflow de Auth.
El ADR-007 conserva su contexto histórico con una nota del estado implementado.
Los requisitos y contratos futuros no se presentan como funcionalidad terminada.

La reorganización agrupa protección, infraestructura Redis y pruebas internas;
ajusta imports, módulos y la ruta de inclusión del script Lua. Incluye el orden
de funciones y comentarios revisados durante la lectura del código. No cambia
la política de autenticación ni añade publicación de revocaciones o consumidores.

## Diferencias documentales corregidas

- El reenvío conserva enlaces anteriores; confirmar uno invalida los pendientes.
- Un fallo del proveedor durante reenvío mantiene 202; el fallo de envío inicial
  se representa mediante `verification_email_sent=false` en la respuesta 201.
- El límite global histórico de 120/minuto ya no existe: hay presupuestos por
  IP/operación y por destinatario/cuenta, con reglas distintas en Nginx y Redis.
- Redis se conecta de forma diferida; sesiones y refresh viven en PostgreSQL.
- El arranque requiere configuración y archivos JWT incluso para usar registro.
- Login/refresh/logout y outbox persistida existen; API privada y RabbitMQ no.
- Los rechazos del extractor JSON de registro/verificación/reenvío no se adaptan
  a Problem Details. El manejo genérico de errores SQLx puede registrar datos
  personales. No se promete redacción completa de todos los logs.
- OpenAPI cubre sesiones/JWKS; registro/verificación/reenvío siguen fuera de él.
- Se actualizaron estructura, filtros de pruebas y ejemplo de error de health/db;
  los resultados de entregas anteriores quedaron identificados como históricos.

## Comprobaciones ejecutadas

Entorno Linux de la VM Vagrant. Dependencias desechables y aisladas: PostgreSQL
`17-alpine`, Redis `8-alpine` y Mailpit `v1.27`, publicados solo en loopback en
puertos dinámicos. Configuración por variables temporales y claves Ed25519
generadas exclusivamente para esta comprobación. No se modificó el `.env` local
ni se usó la base de desarrollo para los datos del recorrido HTTP.

Desde `services/auth`, con las variables de ese entorno:

```sh
cargo fmt --check
cargo check --all-targets --locked
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked -- --include-ignored
cargo build --locked
```

Resultado: formato, compilación y Clippy estricto correctos. **83 pruebas
correctas, cero fallos y cero ignoradas**: 8 de biblioteca, 72 del binario y
3 de integración externa. Incluye los tests Redis y comprobaciones Unix de claves.

Desde la raíz:

```sh
python3 scripts/test-auth-jwks-gateway.py --nginx /usr/sbin/nginx
```

Resultado: prueba existente con Nginx real correcta; sintaxis, JWKS GET/HEAD,
caché, presupuestos separados y 404. Ese script utiliza un backend stub.

Adicionalmente se ejecutó un recorrido con el binario real de Auth y copias
temporales de la configuración Nginx del repositorio, cambiando puertos,
upstream, rutas de archivos y logs. El proxy confiable de esa comprobación fue
loopback. Se verificó:

1. Health de PostgreSQL/Redis y JWKS a través de Nginx.
2. Registro 201 con correo recibido en Mailpit; registro duplicado 409.
3. HEAD del enlace 405 sin consumo; reenvío durante cooldown 202 sin otro correo.
4. Reenvío elegible 202 y recepción de un segundo correo. Para evitar la espera
   de un minuto se envejeció `created_at` únicamente en la base desechable.
5. El enlace original siguió funcionando mediante GET después del reenvío.
6. Tras confirmar, el enlace alternativo y la reutilización devolvieron 400.
7. Registro de otra cuenta y confirmación POST 200 con `email_verified=true`.
8. PostgreSQL confirmó exactamente dos cuentas verificadas.
9. Una ráfaga produjo 429 con `Retry-After`.

Los procesos, contenedores y claves temporales de esa comprobación se limpiaron
al terminar. Se instaló Nginx en la VM para ejecutar estas comprobaciones.

También se comprobó la sintaxis del firewall con `nft -c`, la unidad
`auth-firewall.service` activa y la tabla instalada: permiso desde `br-auth` /
`172.30.250.2` al puerto 3001 y rechazo para las demás conexiones a ese puerto.
Esta lectura no sustituye una prueba de tráfico por la topología desplegada;
el recorrido aislado utilizó otros puertos y no acredita ese bloqueo con paquetes.

## Límites y pendientes

- No se comprobó entrega real con Resend ni un despliegue HTTPS de producción.
- No se acredita ejecución de CI remoto ni interoperabilidad Rust–Elixir.
- Login sigue deshabilitado por defecto por #17. Sus pruebas pasan en bases
  aisladas; eso no resuelve el problema de seguridad descrito en SESSIONS.md.
- Pendientes de consolidación: redacción de errores SQL, contrato OpenAPI de
  registro/verificación/reenvío y uniformidad de errores de extractores JSON.
- La suite actual no demuestra todos los escenarios posibles de registro,
  vencimiento, cambio de correo o recuperación; la documentación de cobertura
  se limita a lo observado en las pruebas existentes y el recorrido ejecutado.

No se modificaron esos comportamientos para hacer coincidir el código con una
promesa documental anterior. Esta revisión documenta el comportamiento actual.
