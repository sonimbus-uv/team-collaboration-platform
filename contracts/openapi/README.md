# Contratos REST

Un archivo OpenAPI por servicio (`auth.yaml`, `core.yaml`, `files.yaml`, `reports.yaml`). Ver las reglas en [`contracts/README.md`](../README.md).

Auth separa [API pública parcial](auth.yaml) y [API interna privada](auth-internal.yaml)
para impedir confundir exposición y credenciales. Ambos son OpenAPI 3.1 en sintaxis
JSON (subconjunto de YAML), contratos de #35. Firma y JWKS están implementados
en la segunda entrega; sesiones y endpoints públicos se implementan en fase 3,
con login deshabilitado por #17. La API interna 4B tiene implementación local y
despliegue pendiente. OpenAPI público 0.2.0 incluye el registro con intento+código
aprobado el 2026-10-28, pendiente de revisión técnica; ver [registration](../registration.md).
Los prefijos de registro y sesiones conservan su diferencia; convergencia en #14.
Ver [flujos](../auth-session-flows.md), [tokens](../token-profile.md) y
[comprobación interna](../session-checks.md) para semántica normativa y propuestas.
