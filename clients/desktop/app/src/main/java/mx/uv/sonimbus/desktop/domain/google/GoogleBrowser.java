package mx.uv.sonimbus.desktop.domain.google;

import java.util.function.BooleanSupplier;

public interface GoogleBrowser extends AutoCloseable {
    String authorize(GoogleAttempt attempt, BooleanSupplier cancelled) throws GoogleException;

    void cancel();

    @Override
    default void close() {
        cancel();
    }
}
