# Desarrollo local de Auth

Los scripts se ejecutan como usuario regular dentro de la VM Vagrant Linux ya
aprovisionada, con acceso a Docker y sudo para systemd. No instalan herramientas,
rotan secretos existentes ni sustituyen la configuración local. No son un
procedimiento de despliegue de producción.

## Preparación

Desde PowerShell, en la raíz del repositorio:

```powershell
vagrant up
vagrant ssh
```

Dentro de la VM:

```bash
cd /vagrant
bash scripts/auth/dev/setup-auth-dev.sh
```

El aprovisionamiento debe aportar `infrastructure/.env`, Rust, Docker Compose,
OpenSSL, Python 3 y `auth-firewall.service`. La preparación resuelve las
credenciales y puertos efectivos de Compose sin ejecutar los `.env` como shell.
Si `services/auth/.env` existe, finaliza sin modificarlo ni repararlo.

Para un entorno nuevo genera:

- `.env` local ignorado por Git, login y contraseñas deshabilitados, y API interna
  habilitada únicamente en loopback.
- Material Ed25519, hashes de credenciales y tokens `core.token` y `files.token`
  en un directorio privado `~/.local/share/sonimbus-auth-dev-*`.
- CA de desarrollo de un año y certificado localhost de 30 días. La renovación
  requiere una acción operativa; repetir la preparación no rota el material.
- Copias de certificados públicos en `secrets/sonimbus-auth-dev-*` para Postman.

Los permisos Linux son `0700` para el directorio y `0600` para sus archivos. Las
ACL del directorio compartido en Windows también deben proteger `.env`. La CA
privada y los tokens no se exportan a Postman ni se incorporan al repositorio.
Un fallo conserva la configuración existente y deja el material generado en el
directorio indicado para su inspección.

## Arranque

Dentro de la VM:

```bash
cd /vagrant
bash scripts/auth/dev/start-auth-dev.sh --enable-login
```

Sin `--enable-login`, el script mantiene login deshabilitado. La activación de
contraseñas es independiente y se configura según
[configuración](configuration.md#habilitación-de-funciones).

El script inicia el firewall instalado y PostgreSQL, Redis, Mailpit y Nginx;
espera su disponibilidad hasta 120 segundos y ejecuta Cargo en primer plano.
Auth aplica las migraciones a la base configurada. Los artefactos se guardan en
`~/.cargo-target/auth-service`, salvo un `CARGO_TARGET_DIR` ya establecido.

`Ctrl+C` detiene Auth; contenedores y firewall permanecen activos. Un segundo
proceso no puede ocupar los mismos puertos. Los scripts no aplican la plantilla
de firewall privado ni alteran la política de red.

## Acceso desde Windows

Un segundo terminal PowerShell mantiene los túneles de desarrollo:

```powershell
vagrant ssh -- -N -L 18080:127.0.0.1:8080 -L 18025:127.0.0.1:8025 -L 18443:127.0.0.1:3443
```

| Dirección con puertos estándar | Servicio |
|---|---|
| `http://localhost:18080/health` | Gateway y Auth |
| `http://localhost:18025` | Mailpit |
| `https://localhost:18443` | API privada |

Si Compose publica otros puertos, el destino del túnel debe ajustarse. Postman
importa `dev-ca.crt` como CA y conserva la verificación TLS habilitada. Los tokens
se leen localmente del directorio privado y se guardan únicamente en valores
locales de Postman, sin exportación ni registro.

Las colecciones pública y privada se ejecutan por separado, una iteración cada
una. La privada requiere ambos tokens y confianza en la CA. La colección
[auth-passwords](../../../postman/automated/auth-passwords.postman_collection.json)
requiere login y contraseñas habilitados, `base_url` y `mailpit_url`; crea un
usuario de prueba. Su ejecución es independiente de las pruebas Rust.

## Diagnóstico del entorno

- El usuario necesita los permisos Docker previstos; ejecutar Auth como root no
  sustituye la configuración del entorno.
- Los volúmenes PostgreSQL conservan sus contraseñas originales; modificar
  `infrastructure/.env` no cambia las credenciales dentro de una base existente.
- La ausencia de `auth-firewall.service` requiere revisar el aprovisionamiento.
- Los errores de claves o certificados requieren revisar los archivos y su vigencia.
- La disponibilidad de contenedores no sustituye `/health/db` y `/health/redis`.

Los comandos reproducibles de validación se mantienen en [pruebas](testing.md).
