package mx.uv.sonimbus.desktop.data.instance;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

class SingleInstanceTest {
    @TempDir Path directory;

    @Test
    void secondaryLaunchActivatesPrimaryWithoutAcquiringAnotherInstance() throws Exception {
        AtomicInteger activations = new AtomicInteger();
        try (SingleInstance primary =
                SingleInstance.acquire(directory, activations::incrementAndGet)) {
            assertNotNull(primary);
            assertNull(SingleInstance.acquire(directory, () -> {}));
            assertEquals(1, activations.get());
        }
        assertFalse(Files.exists(directory.resolve("client.endpoint")));
    }

    @Test
    void newPrimaryCanStartAfterThePreviousOneCloses() throws Exception {
        SingleInstance first = SingleInstance.acquire(directory, () -> {});
        first.close();
        try (SingleInstance second = SingleInstance.acquire(directory, () -> {})) {
            assertNotNull(second);
        }
    }

    @Test
    void staleEndpointDoesNotPreventRecoveryWhenNoProcessHoldsTheLock() throws Exception {
        Files.writeString(directory.resolve("client.endpoint"), "stale");
        try (SingleInstance primary = SingleInstance.acquire(directory, () -> {})) {
            assertNotNull(primary);
            assertFalse(Files.readString(directory.resolve("client.endpoint")).contains("stale"));
        }
    }

    @Test
    void separateJvmAlsoActivatesTheExistingClient() throws Exception {
        AtomicInteger activations = new AtomicInteger();
        try (SingleInstance primary =
                SingleInstance.acquire(directory, activations::incrementAndGet)) {
            Process secondary =
                    new ProcessBuilder(
                                    Path.of(System.getProperty("java.home"), "bin", "java")
                                            .toString(),
                                    "-cp",
                                    System.getProperty("java.class.path"),
                                    SingleInstanceTest.class.getName(),
                                    directory.toString())
                            .redirectErrorStream(true)
                            .start();
            try {
                assertTrue(secondary.waitFor(8, TimeUnit.SECONDS));
                assertEquals(0, secondary.exitValue());
                assertEquals(
                        "SECONDARY", new String(secondary.getInputStream().readAllBytes()).trim());
                assertEquals(1, activations.get());
            } finally {
                secondary.destroyForcibly();
            }
        }
    }

    public static void main(String[] args) throws Exception {
        try (SingleInstance instance = SingleInstance.acquire(Path.of(args[0]), () -> {})) {
            System.out.println(instance == null ? "SECONDARY" : "PRIMARY");
        }
    }
}
