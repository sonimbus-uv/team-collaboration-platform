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
También prepara TLS del gateway y aplica `infrastructure/compose.auth-tls.yaml`.
Auth aplica las migraciones a la base configurada. Los artefactos se guardan en
`~/.cargo-target/auth-service`, salvo un `CARGO_TARGET_DIR` ya establecido.

`Ctrl+C` detiene Auth; contenedores y firewall permanecen activos. Un segundo
proceso no puede ocupar los mismos puertos. Los scripts no aplican la plantilla
de firewall privado ni alteran la política de red.

## Acceso desde Windows

Un segundo terminal PowerShell mantiene los túneles de desarrollo, vinculados
únicamente a loopback de Windows:

```powershell
.\scripts\auth\dev\connect-auth-dev.ps1
```

| Dirección con puertos estándar | Servicio |
|---|---|
| `https://localhost:8443/health` | Gateway público con TLS |
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


## HTTPS del gateway y callback Google

La CA del gateway es independiente de la API interna. El script
`setup-gateway-tls.py` conserva CA y claves exclusivamente en
`~/.local/share/sonimbus-gateway-dev`, con directorio `0700` y archivos `0600`.
Nginx recibe solo su certificado y clave de servidor, mediante montajes de solo
lectura. La exportación `secrets/gateway-dev/ca.crt` contiene únicamente la CA
pública y permanece ignorada por Git.

La CA dura un año; el certificado de servidor dura 30 días e incluye `localhost`
y `127.0.0.1`. Cada arranque conserva certificados válidos y renueva el de
servidor cuando quedan menos de 24 horas o no supera la validación. Una CA con
menos de 30 días restantes exige renovación operativa explícita; el script no
sustituye silenciosamente la raíz de confianza. Tras renovar el certificado,
Nginx debe reiniciarse para cargarlo.

Para preparar o activar solo el gateway, sin iniciar Auth ni aplicar migraciones,
se ejecuta dentro de la VM:

```bash
cd /vagrant
python3 scripts/auth/dev/setup-gateway-tls.py
export GATEWAY_TLS_DIR="$HOME/.local/share/sonimbus-gateway-dev"
docker compose --env-file infrastructure/.env \
  -f infrastructure/compose.yaml -f infrastructure/compose.auth-tls.yaml \
  up -d --no-deps --force-recreate nginx
docker compose --env-file infrastructure/.env \
  -f infrastructure/compose.yaml -f infrastructure/compose.auth-tls.yaml \
  exec -T nginx nginx -t
```

Para confiar en esta CA en el almacén del usuario actual de Windows:

```powershell
.\scripts\auth\dev\trust-gateway-dev-ca.ps1
.\scripts\auth\dev\connect-auth-dev.ps1
```

El primer comando agrega una raíz de confianza de desarrollo al usuario actual;
no modifica el almacén de equipo. No se ejecuta con certificados de terceros.
Para retirarla, se elimina de `Cert:\CurrentUser\Root` la huella mostrada por
el script. Postman y clientes con almacén propio importan esta misma CA pública,
manteniendo habilitada la validación TLS.

Google debe registrar exactamente esta URI de redireccionamiento autorizada:

```text
https://localhost:8443/api/v1/auth/google/callback
```

`AUTH_GOOGLE_CALLBACK_URL` utiliza el mismo valor. El cliente inicia el recorrido
del navegador en este origen HTTPS para que la cookie segura de correlación
regrese al callback. La API interna `https://localhost:18443` no es un callback
público. El puerto TLS de Compose se publica solo en loopback de la VM y el túnel
lo lleva a loopback de Windows; no se agrega exposición en la red local.

El Compose base conserva HTTP para las herramientas existentes. El acceso OAuth
se realiza por HTTPS desde el inicio; este ajuste es de desarrollo y no configura
un dominio ni certificados de producción.


## Prueba manual de Google con una cuenta real

Con Google habilitado en Auth, las credenciales configuradas, el callback
registrado y el túnel HTTPS activo, Windows puede ejecutar:

```powershell
python scripts/auth/dev/smoke-google-login.py
```

El receptor temporal usa `sonimbus://auth/google`, por lo que el destino
`desktop` debe conservar esa URL en `AUTH_GOOGLE_RETURN_TARGETS`. El script se
niega a reemplazar un manejador `sonimbus` existente. Si no existe, registra uno
para el usuario actual durante la prueba y lo elimina al terminar. La aplicación
de escritorio mantiene su integración independiente de este receptor.

La selección de cuenta y el consentimiento se realizan en el navegador del
sistema. Cuando este solicita abrir Sonimbus, se permite la apertura del receptor
local. El script conserva el secreto del intento, PKCE y los tokens solo en
memoria; comprueba exchange, sesión autenticada, rechazo del canje repetido,
renovación y revocación. Cierra la sesión de prueba al terminar. La cuenta creada
permanece en la base de desarrollo; el login puede reemplazar una sesión desktop
previa de esa misma cuenta, según la política habitual del servicio.

La prueba no solicita contraseñas de Google en terminal ni imprime tokens.
Un correo ya verificado en otra cuenta Sonimbus produce `account-link-required`;
ese caso requiere el flujo explícito de vinculación desde la cuenta existente.
El receptor espera hasta nueve minutos. Debe permitirse que finalice para retirar
su registro temporal; no se utiliza como manejador permanente de escritorio.


Si se requiere vincular una cuenta existente, la ejecución debe solicitarlo
explícitamente:

```powershell
python scripts/auth/dev/smoke-google-login.py --link
```

Este modo abre un diálogo local para las credenciales de Sonimbus, inicia una
sesión con contraseña y solicita la vinculación autenticada. Después comprueba
la conexión, cierra esa sesión y abre un nuevo recorrido Google, verificando que
el login corresponde al mismo `account_id`. El correo y la contraseña no se
escriben en archivos ni argumentos de procesos. La vinculación permanece en la
cuenta; solo se cierran las sesiones creadas por la prueba. Si Google presenta
selector de cuentas en ambos recorridos, debe elegirse la misma identidad.
