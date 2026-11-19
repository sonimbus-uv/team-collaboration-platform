package mx.uv.sonimbus.desktop.data.browser.google;

import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.domain.google.GoogleCallback;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Duration;
import java.util.UUID;

public final class GoogleCallbackLauncher {
    private GoogleCallbackLauncher() {}

    public static void main(String[] args) {
        try {
            if (args.length != 1) {
                return;
            }
            boolean delivered = forward(URI.create(args[0]), GoogleCallbackChannel.directory());
            AuthLog.googleStage(
                    delivered
                            ? AuthLog.GoogleStage.RETURN_DELIVERED
                            : AuthLog.GoogleStage.RETURN_UNAVAILABLE);
        } catch (Exception failure) {
            AuthLog.unexpectedFailure("google-callback-delivery", failure);
        }
    }

    public static boolean forward(URI uri, Path directory) throws Exception {
        UUID attempt = UUID.fromString(GoogleCallback.parameters(uri).get("attempt_id"));
        Path endpoint = directory.resolve(attempt + ".endpoint");
        if (!Files.isRegularFile(endpoint)
                || Files.isSymbolicLink(endpoint)
                || Files.size(endpoint) > 128) {
            return false;
        }
        String[] values = Files.readString(endpoint).split("\n", -1);
        if (values.length != 2 || !values[1].matches("[A-Za-z0-9_-]{43}")) {
            return false;
        }
        int port = Integer.parseInt(values[0]);
        if (port < 1 || port > 65535) {
            return false;
        }
        HttpRequest request =
                HttpRequest.newBuilder(URI.create("http://127.0.0.1:" + port + "/callback"))
                        .timeout(Duration.ofSeconds(3))
                        .header("Authorization", "Bearer " + values[1])
                        .POST(HttpRequest.BodyPublishers.ofString(uri.toString()))
                        .build();
        try (HttpClient client =
                HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(3)).build()) {
            return client.send(request, HttpResponse.BodyHandlers.discarding()).statusCode() == 204;
        }
    }
}
