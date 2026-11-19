package mx.uv.sonimbus.desktop.domain.google;

import java.time.Instant;
import java.util.Objects;

public record GoogleConnection(String providerEmail, Instant linkedAt) {
    public GoogleConnection {
        Objects.requireNonNull(linkedAt);
    }

    @Override
    public String toString() {
        return "GoogleConnection[REDACTED]";
    }
}
