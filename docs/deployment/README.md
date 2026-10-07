# Despliegue

Procedimientos de despliegue fuera del entorno de desarrollo: servidor, puertos expuestos, certificados y respaldos. El entorno de desarrollo (VM de Vagrant) está descrito en el [README](../../README.md) y en el [ADR-006](../decisions/ADR-006-vagrant-docker-compose.md).

## Servidor de la expo (D-10, D-14)

- Laptop de Guillermo: ASUS TUF, 16 GB de RAM, Ubuntu nativo.
- Se usa el mismo `infrastructure/compose.yaml` que en desarrollo, con un perfil de producción y los secretos fuera del repositorio.
- Migración posterior a un VPS: copiar el `compose`, el `.env` y los volúmenes.

## Recursos estimados

| Componente | RAM aproximada |
|---|---|
| ScyllaDB (1 nodo, memoria acotada) | 1–1.5 GB |
| PostgreSQL | 200–400 MB |
| RabbitMQ | 150–300 MB |
| LiveKit | 100–300 MB |
| MinIO | 150–250 MB |
| Servicios Elixir | 300–500 MB en total |
| Auth (Rust), Redis y Nginx | Menos de 150 MB |
| **Total** | **Unos 2.5–3.5 GB** |

La estimación original de 3–4.5 GB suponía Cassandra con 1 GB de *heap*; con ScyllaDB (D-05) baja. Debe confirmarse con una medición real (T-09 en [`docs/PLAN.md`](../PLAN.md)).

## Checklist de la expo

- [ ] Perfil de producción del `compose`, con secretos fuera del repositorio.
- [ ] Laptop servidor con IP fija, suspensión desactivada, conectada a la corriente y `ufw` abriendo solo el 443, el puerto TCP de LiveKit y su rango UDP.
- [ ] Router o hotspot propio: el Wi-Fi de la universidad puede aislar clientes o bloquear el UDP que LiveKit necesita.
- [ ] TLS: si los certificados son autofirmados (`mkcert`), instalar la autoridad en los clientes.
- [ ] Respaldos (`pg_dump` y `nodetool snapshot` de ScyllaDB) y restauración probada.
- [ ] Guía de operación: levantar, verificar salud, reiniciar y restaurar.
- [ ] Ensayo completo de la demostración.
- [ ] Plan de migración a un VPS.
