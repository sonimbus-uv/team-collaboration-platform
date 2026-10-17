# ADR-006: Uso de Vagrant y Docker Compose para el entorno de desarrollo

## Estado

Aceptado.

## Fecha

2026-09-17.

## Contexto

El proyecto requiere un entorno GNU/Linux reproducible para ejecutar los servicios del backend y sus dependencias mediante contenedores Docker.

El equipo utiliza computadoras con configuraciones diferentes. Instalar manualmente Docker, PostgreSQL, Cassandra, Redis, MinIO y las demás dependencias podría producir incompatibilidades y dificultar la reproducción de errores.

También es necesario controlar el consumo de recursos, ya que la computadora con menores prestaciones dispone de 16 GB de RAM.

## Decisión

Se utilizará Vagrant para crear una máquina virtual Ubuntu Server 24.04 LTS.

El `Vagrantfile` admite dos proveedores y cada integrante elige uno según su sistema:

- **VirtualBox** (Windows, macOS y Linux), con la box `bento/ubuntu-24.04`.
- **libvirt/KVM** (solo Linux), con la box `cloud-image/ubuntu-24.04`, ya que `bento` no se publica para libvirt, y la carpeta del proyecto compartida mediante virtiofs.

*Actualización del 2026-10-17:* originalmente solo se contemplaba VirtualBox; se agregó libvirt para quienes trabajan en Linux con KVM.

La máquina tendrá:

- 6 GB de RAM;
- 4 CPU virtuales;
- disco principal de 80 GB;
- IP privada `192.168.33.30`.

Docker Engine y Docker Compose serán instalados automáticamente mediante `scripts/provision.sh`.

Los servicios predeterminados serán:

- PostgreSQL;
- Redis;
- Mailpit.

Cassandra se ejecutará mediante el perfil `messaging`.

MinIO se ejecutará mediante el perfil `storage`.

Los datos persistentes se almacenarán mediante volúmenes nombrados de Docker.

La configuración local se guardará en `infrastructure/.env`, mientras que el repositorio conservará únicamente `infrastructure/.env.example`.

## Alternativas consideradas

### Máquina virtual configurada manualmente

Se descartó porque cada integrante podría terminar con versiones y configuraciones diferentes.

### Instalación directa en el sistema anfitrión

Se descartó porque produciría diferencias entre Windows y GNU/Linux y no cumpliría completamente con el entorno virtualizado requerido.

### Docker Desktop sin máquina virtual administrada por el proyecto

Se descartó porque dependería de la configuración particular de cada integrante y reduciría la equivalencia con el entorno GNU/Linux de despliegue.

## Consecuencias positivas

- Entorno reproducible para todo el equipo.
- Menor dependencia de configuraciones personales.
- Instalación automatizada.
- Servicios aislados mediante contenedores.
- Persistencia mediante volúmenes.
- Posibilidad de iniciar únicamente los servicios necesarios mediante perfiles.

## Consecuencias negativas

- Mayor consumo de RAM y almacenamiento.
- Dependencia de Vagrant y de un proveedor (VirtualBox o libvirt).
- Dos boxes distintas según el proveedor; los cambios en el `Vagrantfile` deben probarse con ambos.
- La primera ejecución requiere descargar la box y las imágenes.
- Cassandra puede tardar varios minutos en estar disponible.
- Los cambios en recursos o disco pueden requerir reiniciar o recrear la VM.

## Consideraciones de seguridad

- `.env` no deberá versionarse.
- `.env.example` no deberá contener credenciales reales.
- Los servicios estarán disponibles mediante una red privada de desarrollo.
- No deberán incorporarse datos de volúmenes al repositorio.
- No deberán utilizarse credenciales de desarrollo en producción.
## Actualización (2026-10-07)

La base de mensajes cambia de Cassandra a ScyllaDB (D-05 en [`README.md`](README.md)); el perfil `messaging` se conserva. Para la expo, el mismo Compose corre en Ubuntu nativo (D-14). El resto de esta decisión sigue vigente.
