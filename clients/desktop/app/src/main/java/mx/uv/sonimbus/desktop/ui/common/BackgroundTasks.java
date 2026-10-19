package mx.uv.sonimbus.desktop.ui.common;

import java.util.Objects;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;
import java.util.concurrent.Executor;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.Consumer;
import javafx.application.Platform;

/**
 * Ejecuta trabajo bloqueante (llamadas a los servicios) fuera del hilo de JavaFX y entrega el
 * resultado de vuelta en él, para que los controladores puedan tocar la interfaz.
 */
public final class BackgroundTasks implements AutoCloseable {

    /** Trabajo que puede lanzar excepciones verificadas, como {@code ApiException}. */
    @FunctionalInterface
    public interface Work<T> {
        T run() throws Exception;
    }

    private final Executor background;
    private final Executor ui;
    private final ExecutorService owned;

    /** Hilos daemon en segundo plano y entrega con {@link Platform#runLater}. */
    public BackgroundTasks() {
        this(Executors.newCachedThreadPool(daemonThreads()), Platform::runLater);
    }

    /** Para pruebas: permite ejecutar todo en el hilo actual. */
    public BackgroundTasks(Executor background, Executor ui) {
        this.background = Objects.requireNonNull(background);
        this.ui = Objects.requireNonNull(ui);
        this.owned = background instanceof ExecutorService service ? service : null;
    }

    /**
     * Ejecuta {@code work} en segundo plano. Después llama, en el hilo de la interfaz,
     * a {@code onSuccess} con el resultado o a {@code onFailure} con la excepción original.
     */
    public <T> void run(Work<T> work, Consumer<? super T> onSuccess, Consumer<? super Throwable> onFailure) {
        CompletableFuture
                .supplyAsync(() -> {
                    try {
                        return work.run();
                    } catch (RuntimeException e) {
                        throw e;
                    } catch (Exception e) {
                        throw new CompletionException(e);
                    }
                }, background)
                .whenCompleteAsync((result, error) -> {
                    if (error == null) {
                        onSuccess.accept(result);
                    } else {
                        onFailure.accept(unwrap(error));
                    }
                }, ui);
    }

    @Override
    public void close() {
        if (owned != null) {
            owned.shutdownNow();
        }
    }

    private static Throwable unwrap(Throwable error) {
        return error instanceof CompletionException && error.getCause() != null ? error.getCause() : error;
    }

    static ThreadFactory daemonThreads() {
        AtomicInteger count = new AtomicInteger();
        return task -> {
            Thread thread = new Thread(task, "sonimbus-background-" + count.incrementAndGet());
            thread.setDaemon(true);
            return thread;
        };
    }
}
