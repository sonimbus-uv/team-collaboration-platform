# ScyllaDB

Esquemas CQL de mensajes y reacciones de Core (D-05), con el diseño de [`docs/database/`](../../docs/database/README.md#scylladb-d-05), numerados en orden de aplicación (`001_keyspace.cql`, `002_messages.cql`, ...). Ver [`database/README.md`](../README.md).

| Archivo | Contenido |
|---|---|
| `001_keyspace.cql` | *Keyspace* `sonimbus_core`: factor de replicación 1, sin *tablets* |
| `002_messages.cql` | `messages_by_channel`: mensajes por canal y mes |
| `003_messages_by_client_id.cql` | `messages_by_client_id`: reserva de `client_message_id` para no duplicar envíos |
| `004_reactions.cql` | `reactions_by_channel`: una fila por mensaje, emoji y usuario |

Todos los scripts usan `IF NOT EXISTS`: se pueden aplicar más de una vez.

## Aplicar

Dentro de la VM, con el contenedor de ScyllaDB en marcha (el nombre del servicio depende de T-01):

```bash
vagrant ssh -c 'cd /vagrant/infrastructure && for f in ../database/scylla/0*.cql; do docker compose exec -T <servicio-scylla> cqlsh < "$f"; done'
```

## Notas

- `001_keyspace.cql` desactiva las *tablets* (`tablets = {'enabled': false}`): con ellas, ScyllaDB no admite LWT, y la idempotencia usa `INSERT ... IF NOT EXISTS`. Esa opción no existe en Cassandra, así que el script falla mientras `compose.yaml` siga con `cassandra:5.0.9` (T-01).
- Probado el 2026-10-28 con `scylladb/scylla:2025.1` (2025.1.16): creación, segunda aplicación, reserva duplicada (`[applied] = False`), historial paginado, borrado lógico y reacciones por rango.
