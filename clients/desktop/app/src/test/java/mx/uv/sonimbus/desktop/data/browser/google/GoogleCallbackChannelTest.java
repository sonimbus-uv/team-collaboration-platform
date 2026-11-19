package mx.uv.sonimbus.desktop.data.browser.google;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;
import static org.junit.jupiter.api.Assertions.assertTrue;

import mx.uv.sonimbus.desktop.domain.google.GoogleCallback;
import mx.uv.sonimbus.desktop.domain.google.GoogleError;
import mx.uv.sonimbus.desktop.domain.google.GoogleException;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.UUID;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;

class GoogleCallbackChannelTest {
    @TempDir Path directory;
    private static final String CODE = "A".repeat(43);

    @Test
    void forwardsNativeReturnToWaitingInstanceAndCleansEndpoint() throws Exception {
        UUID id = UUID.randomUUID();
        Path endpoint = directory.resolve(id + ".endpoint");
        try (var channel = new GoogleCallbackChannel(id, directory)) {
            assertTrue(GoogleCallbackLauncher.forward(callback(id), directory));
            assertEquals(CODE, channel.result().get(3, TimeUnit.SECONDS));
            assertFalse(GoogleCallbackLauncher.forward(callback(id), directory));
        }
        assertFalse(Files.exists(endpoint));
    }

    @Test
    void immediateBrowserCleanupStillAcknowledgesTheNativeReturn() throws Exception {
        for (int iteration = 0; iteration < 12; iteration++) {
            UUID id = UUID.randomUUID();
            try (var channel = new GoogleCallbackChannel(id, directory)) {
                var cleanup = channel.result().whenComplete((value, failure) -> channel.close());
                assertTrue(GoogleCallbackLauncher.forward(callback(id), directory));
                assertEquals(CODE, cleanup.get(3, TimeUnit.SECONDS));
            }
        }
    }

    @Test
    void rejectsUnauthenticatedLocalRequestsWithoutConsumingResult() throws Exception {
        UUID id = UUID.randomUUID();
        try (var channel = new GoogleCallbackChannel(id, directory)) {
            String[] endpoint = Files.readString(directory.resolve(id + ".endpoint")).split("\n");
            HttpRequest request =
                    HttpRequest.newBuilder(
                                    URI.create("http://127.0.0.1:" + endpoint[0] + "/callback"))
                            .POST(HttpRequest.BodyPublishers.ofString(callback(id).toString()))
                            .build();
            try (HttpClient client = HttpClient.newHttpClient()) {
                assertEquals(
                        403,
                        client.send(request, HttpResponse.BodyHandlers.discarding()).statusCode());
            }
            assertFalse(channel.result().isDone());
            assertTrue(GoogleCallbackLauncher.forward(callback(id), directory));
        }
    }

    @Test
    void cancellationCompletesWithFiniteError() throws Exception {
        UUID id = UUID.randomUUID();
        try (var channel = new GoogleCallbackChannel(id, directory)) {
            assertTrue(
                    GoogleCallbackLauncher.forward(
                            URI.create(
                                    "sonimbus://auth/google?attempt_id=" + id + "&error=cancelled"),
                            directory));
            var failed =
                    assertThrows(
                            ExecutionException.class,
                            () -> channel.result().get(3, TimeUnit.SECONDS));
            assertEquals(GoogleError.CANCELLED, ((GoogleException) failed.getCause()).kind());
        }
    }

    @Test
    void rejectsSpoofedTargetsDuplicatesAndUnrelatedProofs() {
        UUID id = UUID.randomUUID();
        for (String uri :
                new String[] {
                    "https://auth/google?attempt_id=" + id + "&completion_code=" + CODE,
                    "sonimbus://evil/google?attempt_id=" + id + "&completion_code=" + CODE,
                    callback(id) + "&attempt_id=" + id,
                    callback(id) + "&access_token=secret",
                    callback(id) + "#fragment",
                    "sonimbus://auth/google?attempt_id=" + id + "&error=arbitrary"
                }) {
            assertThrows(GoogleException.class, () -> GoogleCallback.parameters(URI.create(uri)));
        }
        assertThrows(
                GoogleException.class,
                () -> GoogleCallback.completion(callback(id), UUID.randomUUID()));
    }

    @Test
    void protocolLaunchCommandQuotesPathsAndDoesNotUseShell() {
        String command = WindowsGoogleProtocol.launchCommand();
        assertTrue(command.startsWith("\""));
        assertTrue(command.endsWith("\"%1\""));
        assertTrue(command.contains(GoogleCallbackLauncher.class.getName()));
        assertFalse(command.contains("cmd.exe"));
        assertFalse(command.contains("powershell"));
    }

    private URI callback(UUID id) {
        return URI.create("sonimbus://auth/google?attempt_id=" + id + "&completion_code=" + CODE);
    }
}
