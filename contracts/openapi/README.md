# Contratos REST

Un archivo OpenAPI por servicio (`auth.yaml`, `core.yaml`, `files.yaml`, `reports.yaml`). Ver las reglas en [`contracts/README.md`](../README.md).

Auth separa [API pública parcial](auth.yaml) y [API interna privada](auth-internal.yaml)
para impedir confundir exposición y credenciales. Ambos son OpenAPI 3.1 en sintaxis
JSON (subconjunto de YAML), contratos de #35. Firma y JWKS están implementados
en la segunda entrega; sesiones y API interna siguen pendientes. Registro y
verificación conservan sus rutas actuales; su contrato completo permanece en #14.
Ver [flujos](../auth-session-flows.md), [tokens](../token-profile.md) y
[comprobación interna](../session-checks.md) para semántica normativa y propuestas.
