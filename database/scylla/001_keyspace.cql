-- Keyspace de mensajes y reacciones de Core (D-05).
-- Desarrollo y expo: un nodo, factor de replicacion 1. La consistencia (ONE)
-- la fija el driver, no el esquema.
-- 'replication_factor' con NetworkTopologyStrategy evita depender del nombre
-- del datacenter.
-- Sin tablets: con tablets, ScyllaDB (2025.1) no admite LWT, y la
-- idempotencia de envio usa INSERT ... IF NOT EXISTS (003). Esta opcion es
-- propia de ScyllaDB: el script falla en Cassandra (T-01).

CREATE KEYSPACE IF NOT EXISTS sonimbus_core
    WITH replication = {
        'class': 'NetworkTopologyStrategy',
        'replication_factor': 1
    }
    AND durable_writes = true
    AND tablets = {'enabled': false};
