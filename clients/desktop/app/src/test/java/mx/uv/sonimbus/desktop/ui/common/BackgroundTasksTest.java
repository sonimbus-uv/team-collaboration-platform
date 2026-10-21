package mx.uv.sonimbus.desktop.ui.common;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;

class BackgroundTasksTest {

    /** Hace de «hilo de la interfaz»: una cola que la prueba vacía en su propio hilo. */
    private final BlockingQueue<Runnable> uiQueue = new ArrayBlockingQueue<>(10);
    private final ExecutorService background = Executors.newSingleThreadExecutor();
    private final BackgroundTasks tasks = new BackgroundTasks(background, uiQueue::add);

    @AfterEach
    void shutdown() {
        tasks.close();
    }

    @Test
    void workRunsInBackgroundAndResultArrivesOnUiThread() throws InterruptedException {
        Thread testThread = Thread.currentThread();
        AtomicReference<Thread> workThread = new AtomicReference<>();
        AtomicReference<Thread> callbackThread = new AtomicReference<>();
        AtomicReference<String> result = new AtomicReference<>();

        tasks.run(() -> {
            workThread.set(Thread.currentThread());
            return "ok";
        }, value -> {
            callbackThread.set(Thread.currentThread());
            result.set(value);
        }, error -> { });
        runNextUiTask();

        assertNotEquals(testThread, workThread.get());
        assertSame(testThread, callbackThread.get());
        assertEquals("ok", result.get());
    }

    @Test
    void checkedExceptionArrivesUnwrapped() throws InterruptedException {
        ApiException failure = new ApiException(ApiErrorKind.TIMEOUT, "register", null);
        AtomicReference<Throwable> received = new AtomicReference<>();
        AtomicReference<Object> success = new AtomicReference<>();

        tasks.run(() -> {
            throw failure;
        }, success::set, received::set);
        runNextUiTask();

        assertSame(failure, received.get());
        assertNull(success.get());
    }

    @Test
    void runtimeExceptionArrivesUnwrapped() throws InterruptedException {
        IllegalStateException failure = new IllegalStateException("boom");
        AtomicReference<Throwable> received = new AtomicReference<>();

        tasks.run(() -> {
            throw failure;
        }, value -> { }, received::set);
        runNextUiTask();

        assertSame(failure, received.get());
    }

    @Test
    void defaultThreadsAreDaemonSoTheyNeverBlockExit() {
        Thread thread = BackgroundTasks.daemonThreads().newThread(() -> { });

        assertTrue(thread.isDaemon());
        assertTrue(thread.getName().startsWith("sonimbus-background-"));
    }

    private void runNextUiTask() throws InterruptedException {
        Runnable task = uiQueue.poll(5, TimeUnit.SECONDS);
        assertTrue(task != null, "No llegó ninguna tarea al hilo de la interfaz");
        task.run();
    }
}
