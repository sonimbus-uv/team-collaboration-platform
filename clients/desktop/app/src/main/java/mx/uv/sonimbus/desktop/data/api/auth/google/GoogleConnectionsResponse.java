package mx.uv.sonimbus.desktop.data.api.auth.google;

import mx.uv.sonimbus.desktop.domain.google.GoogleConnection;

import java.time.Instant;
import java.util.List;
import java.util.Objects;
import java.util.Optional;

public record GoogleConnectionsResponse(List<Connection> connections) {
    public record Connection(String provider, String providerEmail, String linkedAt) {
        @Override
        public String toString() {
            return "Connection[REDACTED]";
        }
    }

    public Optional<GoogleConnection> toConnection() {
        Objects.requireNonNull(connections);
        if (connections.size() > 1) {
            throw new IllegalArgumentException("Invalid connections response");
        }
        if (connections.isEmpty()) {
            return Optional.empty();
        }
        Connection connection = connections.getFirst();
        if (!"google".equals(connection.provider())) {
            throw new IllegalArgumentException("Invalid connection provider");
        }
        return Optional.of(
                new GoogleConnection(
                        connection.providerEmail(), Instant.parse(connection.linkedAt())));
    }

    @Override
    public String toString() {
        return "GoogleConnectionsResponse[REDACTED]";
    }
}
