# LiveKit

**Responsable principal:** `@tuzc0`

## Responsabilidad

Configuración del servidor de medios LiveKit autoalojado para las reuniones de voz.

## Notas de la prueba de concepto

- Se validó con `livekit/livekit-server:v1.13.7`.
- La guía oficial recomienda red del anfitrión (`network_mode: host`) en Docker.
- En modo de desarrollo (`--dev`) el servidor usa un único puerto UDP (7882), no el rango 50000–60000. La configuración de despliegue debe definir los puertos explícitamente.
- Las claves del modo de desarrollo (`devkey`/`secret`) no deben usarse fuera de desarrollo; las reales van en `infrastructure/.env`.

## Estado

Sin configuración. El primer PR agrega el archivo de configuración y el servicio en Compose.
