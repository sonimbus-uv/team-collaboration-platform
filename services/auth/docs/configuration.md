# Configuración de Auth

[.env.example](../.env.example) contiene el inventario de variables y sus valores
de referencia. Esta guía describe las condiciones de uso; no mantiene una copia
completa del archivo. Los ejemplos no contienen credenciales utilizables.

## Carga y validación

El proceso carga `.env` desde el entorno de ejecución y después construye `Config`.
Las variables ya exportadas tienen precedencia. Un archivo ausente es admisible
si el entorno aporta los valores requeridos; un archivo mal formado detiene el
arranque. Los booleanos solo aceptan `true` o `false`.

La configuración inválida detiene el inicio sin registrar valores de secretos,
contenido de archivos ni diagnósticos completos del parser. Los cambios requieren
reinicio; no hay recarga dinámica. La configuración y el material JWT se validan
antes de abrir la conexión pública a PostgreSQL. La API interna habilitada valida
sus archivos y prepara su listener antes de servir solicitudes.

## Proceso y dependencias

| Grupo | Variables principales | Condiciones |
|---|---|---|
| Listener público | `SERVER_ADDR` | Dirección y puerto disponibles en el entorno de ejecución |
| PostgreSQL | `DATABASE_URL`, `DATABASE_MAX_CONNECTIONS` | URL del primario autoritativo; máximo de conexiones mayor que cero |
| Redis público | `REDIS_URL`, `REDIS_CONNECT_TIMEOUT_MS`, `REDIS_OPERATION_TIMEOUT_MS` | URL no vacía; timeouts entre 1 y 5000 ms |
| Proxy público | `TRUSTED_PROXY_IPS`, `REQUIRE_TRUSTED_PROXY` | IP exacta, sin puerto ni CIDR; exigir proxy requiere una lista no vacía |
| Hash de contraseña | `MAX_CONCURRENT_HASHES` | Entre 1 y 64 trabajos simultáneos, compartidos por los flujos |
| Correo | `EMAIL_PROVIDER`, `EMAIL_FROM` | Proveedor `mailpit` o `resend`; `resend` es el predeterminado |

Auth se ejecuta como proceso en la VM de desarrollo; el Compose actual no define
un contenedor Auth. Las URL locales usan los puertos publicados de PostgreSQL y
Redis. Un proceso dentro de una red de contenedores debe usar los nombres y
puertos internos de esa red, sin copiar direcciones de otro entorno.

Mailpit utiliza `MAILPIT_SMTP_HOST` y `MAILPIT_SMTP_PORT`, con puerto mayor que
cero. SMTP local de desarrollo no usa autenticación ni TLS; no corresponde a una
configuración de producción. Resend requiere `RESEND_API_KEY`. Ambos transportes
implementan el puerto de correo y seleccionan contenido distinto para registro y
recuperación. `EMAIL_VERIFICATION_URL` ya no participa en el registro con códigos.

## Habilitación de funciones

| Variable | Predeterminado | Efecto |
|---|---|---|
| `AUTH_LOGIN_ENABLED` | `false` | Habilita login; no controla refresh ni logout |
| `AUTH_PASSWORDS_ENABLED` | `false` | Habilita cambio y recuperación; exige secreto independiente |
| `AUTH_INTERNAL_ENABLED` | `false` | Habilita el listener HTTPS privado; exige configuración completa |

El script de preparación local establece la API interna en loopback. El script
de arranque mantiene login apagado salvo presencia de `--enable-login`, incluso
si `.env` lo habilita. Las banderas son independientes. Las migraciones se
aplican al iniciar Auth aunque la función correspondiente esté deshabilitada.

La habilitación en producción requiere revisión de cuentas heredadas,
enumeración, contratos de clientes y condiciones de operación. La documentación
no acredita que esas revisiones ni el despliegue estén completados.

## Secretos

`AUTH_REGISTRATION_CODE_HMAC_SECRET` y `AUTH_RECIPIENT_HMAC_SECRET` son obligatorios.
`AUTH_PASSWORD_RESET_HMAC_SECRET` se exige al habilitar contraseñas y debe diferir
de los dos anteriores. Cada valor requiere al menos 32 bytes y contenido no vacío;
el aprovisionamiento debe aportar entropía criptográfica, no una frase predecible.

La generación puede realizarse fuera del repositorio con `openssl rand -hex 32`.
Los valores se almacenan únicamente en el entorno protegido o en el gestor de
secretos. Todas las réplicas de Auth deben compartir los secretos y las políticas
que coordinan. La rotación del HMAC de registro o recuperación invalida códigos
pendientes; la del destinatario cambia los identificadores de sus presupuestos.

## JWT y archivos de claves

Son obligatorias `AUTH_JWT_PRIVATE_KEY_PATH`, `AUTH_JWT_JWKS_PATH`,
`AUTH_JWT_ACTIVE_KID`, `AUTH_JWT_ISSUER` y `AUTH_JWT_AUDIENCES`.
El emisor debe ser HTTPS y no contener credenciales, query ni fragmento; las
audiencias son identificadores distintos separados por comas, sin espacios.

La privada es Ed25519 PKCS#8 PEM sin contraseña. El JWKS contiene la pública
activa y las retenidas, nunca material privado. Los archivos son regulares, sin
symlinks y de hasta 64 KiB; el conjunto admite entre 1 y 16 claves y rechaza
campos, curvas, algoritmos, codificaciones o identificadores incompatibles.
`kid` admite entre 1 y 128 caracteres ASCII alfanuméricos, punto, guion o guion bajo.

En Unix la privada pertenece al usuario de Auth, sin acceso de grupo u otros
(`0400` o `0600`). En Windows corresponde establecer ACL equivalentes; el cargador
no comprueba ACL de Windows. Los directorios y las escrituras al JWKS también
requieren protección. El arranque no genera claves de respaldo ni de producción.
La rotación se describe en [operación](operations.md#claves-jwt-y-jwks).

## API interna

`AUTH_INTERNAL_ADDR`, `AUTH_INTERNAL_ALLOWED_CIDRS`,
`AUTH_INTERNAL_CREDENTIALS_PATH`, `AUTH_INTERNAL_CERT_PATH` y
`AUTH_INTERNAL_KEY_PATH` son obligatorios cuando la API privada está habilitada.
La dirección de ejemplo es loopback; el acceso remoto exige dirección privada,
CIDR reales de origen, DNS/SAN, certificado y confianza en cada consumidor.

Credenciales y clave TLS son archivos regulares, sin symlinks y de hasta 64 KiB.
En Unix pertenecen al usuario de Auth con permisos restringidos. Los CIDR vacíos
o universales, puerto cero, archivos inválidos, pareja de certificado/clave
inconsistente y presupuestos incompatibles impiden arrancar. No hay fallback HTTP.

El [ejemplo de credenciales](../internal-credentials.example.json) contiene
marcadores inválidos deliberadamente. La distribución y rotación se describen en
[operación](operations.md#credenciales-de-servicio).

## Presupuestos

Las variables `AUTH_*_CAPACITY` y `AUTH_*_REFILL_SECS` configuran los token buckets
públicos; el periodo corresponde a una recarga completa, no a una ventana fija.
Las variables `AUTH_INTERNAL_*` controlan cuotas, reservas, cuerpo y deadlines
privados. El inventario y los predeterminados se mantienen en
[.env.example](../.env.example); las reglas privadas observables pertenecen al
[contrato interno](../../../contracts/auth/internal-api.md#autenticación-y-límites).
Las políticas específicas de contraseña se contrastan con el
[contrato de contraseñas](../../../contracts/auth/public-api.md#presupuestos).
