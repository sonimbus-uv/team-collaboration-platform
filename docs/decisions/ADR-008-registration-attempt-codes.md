# ADR-008: códigos ligados a intentos independientes de registro

## Estado

Flujo y parámetros aprobados por el solicitante el 2026-10-28; implementación
local pendiente de revisión del equipo. No cierra #17 ni habilita login.
Desarrolla RF-AUTH-07/CU-02 y precisa la condición de seguridad de ADR-007.

## Diagnóstico

Se consultó el issue #17 real el 2026-10-28: abierto, sin comentarios. Describe
el bloqueo por `accounts_email_unique` cuando un tercero registra un correo
ajeno. Además, el enlace anterior podía activar la contraseña de ese tercero.
Cambiar el enlace por un código comprobado solo por correo no resuelve ese riesgo.

## Decisión aprobada

Crear una tabla pequeña `auth.registration_attempts`, sin crear una cuenta al
solicitar registro. Cada intento tiene UUID v4, correo ASCII normalizado, hash
Argon2id de la contraseña de esa solicitud y MAC del código. Coexisten hasta
cinco intentos pendientes utilizables por correo. Crear otro no sustituye ni
invalida los existentes. El dueño debe iniciar su propio intento y confirmar
el código correspondiente en esa misma aplicación.

- Código CSPRNG de ocho dígitos, incluidos ceros iniciales; UUID no sustituye código.
- Vencimiento absoluto a los diez minutos del registro; el reenvío no lo prolonga.
- Cinco fallos persistidos por intento; se confirman en DB antes de responder 400.
- Tres reenvíos como máximo, separados por al menos 60 segundos. Cada uno cambia
  código y versión sin reiniciar fallos. El código anterior deja de servir.
- Registro y reenvío comparten token bucket de diez envíos por diez minutos por
  destinatario, además de presupuestos IP existentes. Se reutiliza Redis.
- El límite de pendientes no expulsa intentos: puede bloquear nuevas solicitudes
  temporalmente si un atacante llena el cupo, pero no invalida un intento legítimo.
- MAC HMAC-SHA256 de dominio/propósito, UUID, versión y código. Se verifica en
  tiempo constante; no se almacena código en claro ni un hash rápido sin clave.
  `AUTH_REGISTRATION_CODE_HMAC_SECRET` es independiente del secreto del limitador,
  contiene al menos 32 bytes aleatorios y se aprovisiona fuera de Git.
- Todas las réplicas usan el mismo secreto. Rotarlo vuelve inutilizables códigos
  pendientes; no se implementan múltiples claves ni renovación de vigencia.

## Atomicidad y compatibilidad

Orden: advisory lock transaccional por correo normalizado, fila de cuenta si
existe, fila de intento. El bloqueo por correo serializa también la ausencia de
cuenta. Luego se reevalúan vigencia, estado y código con reloj de PostgreSQL.
La confirmación ganadora crea la cuenta o activa una antigua pendiente con el
hash de SU intento, marca verificación, revoca sesiones heredadas, consume el
intento e invalida los demás y los tokens antiguos de verificación. Todo sucede
en una transacción. Una cuenta verificada o bloqueada no puede ser reemplazada.
Los cambios de credenciales respetan el bloqueo de cuenta previo a sesiones.

La migración 0005 solo crea tabla/índice; no actualiza ni elimina datos heredados.
El GET anterior responde 410 informativo sin consultar ni consumir tokens. El
POST con token y reenvío por correo dejan de ser válidos. No hay activación
alternativa. Cuentas verificadas por el flujo vulnerable requieren auditoría y
decisión de saneamiento independiente: esta solución no las declara seguras.

El envío ocurre después del commit, sin transacción abierta. Un fallo conserva
el intento y comunica `verification_email_sent=false`; admite reenvío sujeto a
los mismos límites. Un envío aceptado no garantiza entrega. No se añade cola ni
reintento automático. Un fallo después del commit y antes de responder puede
dejar un intento cuyo identificador el cliente desconoce; podrá iniciar otro
sin modificar credenciales de cuentas verificadas y sujeto a los cupos.

## Contrato y alternativas

Ver [contrato de registro](../../contracts/registration.md). Se conserva 409 para
cuentas ya verificadas, una señal de enumeración existente y explícitamente
aprobada; no se promete tiempo de respuesta uniforme ni anonimato de existencia.
No se eligió establecer de nuevo la contraseña en una página, purgar cuentas
automáticamente ni convertir el código en recuperación de contraseña.

No se cambian JavaFX/Flutter, Core/Files, RabbitMQ ni `AUTH_LOGIN_ENABLED=false`.
CU-05/forgot/reset permanece pendiente y debe usar un propósito separado.
