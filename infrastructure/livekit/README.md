# LiveKit

**Responsable principal:** `@tuzc0`

## Responsabilidad

Configuración del servidor de medios LiveKit autoalojado para las reuniones de voz.

## Notas de la prueba de concepto

- Se validó con `livekit/livekit-server:v1.13.7`.
- La guía oficial recomienda red del anfitrión (`network_mode: host`) en Docker.
- En modo de desarrollo (`--dev`) el servidor usa un único puerto UDP (7882), no el rango 50000–60000. La configuración de despliegue debe definir los puertos explícitamente.
- Las claves del modo de desarrollo (`devkey`/`secret`) no deben usarse fuera de desarrollo; las reales van en `infrastructure/.env`.

## Configuración

- Servicio `livekit` en [`compose.yaml`](../compose.yaml), perfil `voice`, con `network_mode: host`.
- Archivo [`livekit.yaml`](livekit.yaml), sin secretos. Define el puerto de señal, el puerto ICE/TCP, el rango UDP y la IP del nodo.
- La clave y el secreto de API salen de `LIVEKIT_API_KEY` y `LIVEKIT_API_SECRET` (en `infrastructure/.env`) y se pasan al contenedor como `LIVEKIT_KEYS`. Core firma los tokens de sala con el mismo par.
- Variables de referencia en [`.env.example`](../.env.example): `LIVEKIT_VERSION`, `LIVEKIT_API_KEY`, `LIVEKIT_API_SECRET` y `LIVEKIT_URL`. El secreto debe tener al menos 32 caracteres.

```bash
docker compose --profile voice up -d livekit
```

## Puertos y firewall

| Puerto | Protocolo | Uso |
|---|---|---|
| 7880 | TCP | Señalización (WebSocket) |
| 7881 | TCP | ICE sobre TCP (alternativa cuando UDP está bloqueado) |
| 50000-50200 | UDP | Medios (voz y cámara) |

El rango UDP es de 200 puertos: cada participante usa uno, y la expo no supera esa cifra con margen. La guía oficial propone 50000-60000 para producción; ampliar `port_range_end` si hace falta.

Reglas de `ufw` en la VM:

```bash
sudo ufw allow 7880/tcp
sudo ufw allow 7881/tcp
sudo ufw allow 50000:50200/udp
```

## IP del nodo

El cliente de escritorio llega a la VM por la red privada `192.168.33.30` (ver el [`Vagrantfile`](../../Vagrantfile)). `rtc.node_ip` es esa IP: es la dirección que LiveKit anuncia en los candidatos ICE. Con `use_external_ip: false` no consulta STUN para descubrir otra. Si la VM cambia de IP, actualizar `node_ip` en `livekit.yaml` y `LIVEKIT_URL` en `.env`.

## Guía rápida: encender y verificar

Desde el checkout principal, nunca desde un worktree:

```bash
cd ~/proyectos/sonimbus/team-collaboration-platform
git checkout feature/infrastructure-livekit
export VAGRANT_DEFAULT_PROVIDER=libvirt
vagrant up
vagrant ssh
```

Dentro de la VM:

```bash
cd /vagrant/infrastructure
uptime
docker ps -a --format '{{.Names}} {{.Status}}'
```

Si `docker ps` tarda más de unos segundos o la carga pasa de 8, ver «Si Docker no responde».

### 1. Arrancar LiveKit

`LIVEKIT_API_KEY` y `LIVEKIT_API_SECRET` deben existir en `infrastructure/.env`. Si faltan, `vagrant provision` agrega las variables nuevas de `.env.example`; reemplazar los valores `change_this_*`.

```bash
docker compose --profile voice up -d livekit
docker compose --profile voice ps livekit
docker compose --profile voice logs --tail 20 livekit
```

El registro debe mostrar `portHttp: 7880`, `rtc.portTCP: 7881`, el rango UDP `50000-50200` y la IP `192.168.33.30`.

### 2. Comprobar puertos

En la VM:

```bash
ss -ltn | grep -E ':(7880|7881)\b'
ss -lun | grep -E ':(5[0-9]{4})\b'
curl -s -o /dev/null -w '%{http_code}\n' http://localhost:7880
```

Desde el anfitrión:

```bash
curl -s -o /dev/null -w '%{http_code}\n' http://192.168.33.30:7880
```

Se espera `200`. Si responde en la VM pero no desde el anfitrión, revisar `ufw` (reglas de la sección «Puertos y firewall»).

### 3. Pendientes de la configuración

1. **Health check.** Resuelto: la imagen trae `busybox` con `wget`; el servicio lo usa contra `http://127.0.0.1:7880`.
2. **IP del nodo por variable.** La imagen acepta `--node-ip` (variable `NODE_IP`). Se mantiene `rtc.node_ip` en `livekit.yaml` por simplicidad; no se cambia salvo que haga falta.
3. **Token real.** Pedir un token a Core (o generarlo con la clave de `.env`) y conectar con el cliente de la PoC o con `meet.livekit.io` en modo Custom contra `ws://192.168.33.30:7880`.

### 4. Medir la memoria (T-09)

Con el servicio en reposo y luego con dos participantes (voz y cámara):

```bash
docker stats --no-stream livekit
free -m
```

Anotar los valores en la tabla de abajo.

| Condición | Memoria de `livekit` | `free -m` usado |
|---|---|---|
| En reposo (2026-11-21) | 13,29 MiB | 589 MB usados de 5925 MB (VM completa) |
| 2 participantes, voz | pendiente | pendiente |
| 2 participantes, voz y cámara | pendiente | pendiente |

### 5. Al terminar

Si todo pasa, commit en la rama de trabajo, alcance `infrastructure`:

```bash
git add infrastructure/compose.yaml infrastructure/.env.example infrastructure/README.md infrastructure/livekit
git commit -m "feat(infrastructure): add livekit service to compose"
```

No incluir `docs/STANDARD_DEVELOPMENT.md`, que tiene cambios sin commitear ajenos a esta tarea.

### Si Docker no responde

Síntoma observado el 2026-11-20: `docker ps` sin respuesta, carga 15–18, procesos `runc init` colgados por el *health check* de `mailpit`, y clientes `docker` antiguos esperando al daemon.

```bash
ps -eo pid,pcpu,etime,args --sort=-pcpu | head -12
sudo journalctl -u docker -n 20 --no-pager
```

Probar en este orden, parando cuando Docker responda:

1. Terminar los clientes `docker` que lleven minutos esperando (`kill <pid>`).
2. `sudo systemctl restart docker` (ya se hizo una vez sin éxito).
3. Reiniciar la VM: `vagrant halt && vagrant up`.
4. Si el problema reaparece, parar `mailpit` (`docker compose stop mailpit`) y volver a medir. Está prohibido `docker compose down -v` y `docker volume rm`.

## Estado

Servicio arrancado en la VM el 2026-11-21: 7880/TCP y 7881/TCP escuchando, respuesta 200 en 7880, `healthy`, IP del nodo `192.168.33.30` y rango ICE 50000-50200. LiveKit no abre los puertos UDP hasta que hay participantes. Falta el token real y la medición con dos participantes.
