package mx.uv.sonimbus.desktop.data.browser.google;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;

import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.data.logging.AuthLog.GoogleStage;
import mx.uv.sonimbus.desktop.domain.google.GoogleCallback;
import mx.uv.sonimbus.desktop.domain.google.GoogleError;
import mx.uv.sonimbus.desktop.domain.google.GoogleException;

import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.Base64;
import java.util.List;
import java.util.UUID;
import java.util.concurrent.CompletableFuture;

public final class GoogleCallbackChannel implements AutoCloseable {
    private final HttpServer server;
    private final Path endpoint;
    private final UUID attemptId;
    private boolean ownsEndpoint;
    private final String secret;
    private final CompletableFuture<String> result = new CompletableFuture<>();

    public GoogleCallbackChannel(UUID attemptId, Path directory) throws IOException {
        this.attemptId = attemptId;
        Files.createDirectories(directory);
        endpoint = directory.resolve(attemptId + ".endpoint");
        byte[] random = new byte[32];
        new SecureRandom().nextBytes(random);
        secret = Base64.getUrlEncoder().withoutPadding().encodeToString(random);
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        try {
            server.createContext("/callback", this::receive);
            publishEndpoint();
            server.start();
        } catch (IOException | RuntimeException failure) {
            server.stop(0);
            if (ownsEndpoint) {
                Files.deleteIfExists(endpoint);
            }
            throw failure;
        }
    }

    private void publishEndpoint() throws IOException {
        mx.uv.sonimbus.desktop.data.local.PrivateLocalFile.create(endpoint);
        ownsEndpoint = true;
        Files.writeString(
                endpoint, server.getAddress().getPort() + "\n" + secret, StandardCharsets.US_ASCII);
    }

    private void receive(HttpExchange request) throws IOException {
        try {
            if (!"POST".equals(request.getRequestMethod())
                    || !"/callback".equals(request.getRequestURI().toString())
                    || !authorized(request)) {
                request.sendResponseHeaders(403, -1);
                return;
            }
            byte[] bytes = request.getRequestBody().readNBytes(2049);
            if (bytes.length > 2048 || result.isDone()) {
                request.sendResponseHeaders(400, -1);
                return;
            }
            URI uri = URI.create(new String(bytes, StandardCharsets.UTF_8));
            if (!attemptId.toString().equals(GoogleCallback.parameters(uri).get("attempt_id"))) {
                request.sendResponseHeaders(400, -1);
                return;
            }
            String completion = null;
            GoogleException callbackFailure = null;
            try {
                completion = GoogleCallback.completion(uri, attemptId);
            } catch (GoogleException cancelledOrFailed) {
                callbackFailure = cancelledOrFailed;
            }
            request.sendResponseHeaders(204, -1);
            request.close();
            // The waiting browser closes this server as soon as the result is delivered.
            if (callbackFailure == null) {
                AuthLog.googleStage(GoogleStage.CALLBACK_RECEIVED);
                result.complete(completion);
            } else {
                AuthLog.googleStage(
                        callbackFailure.kind() == GoogleError.CANCELLED
                                ? GoogleStage.CALLBACK_CANCELLED
                                : GoogleStage.CALLBACK_REJECTED);
                result.completeExceptionally(callbackFailure);
            }
        } catch (IllegalArgumentException | GoogleException invalidCallback) {
            request.sendResponseHeaders(400, -1);
        } finally {
            request.close();
        }
    }

    private boolean authorized(HttpExchange request) {
        List<String> values = request.getRequestHeaders().get("Authorization");
        return values != null
                && values.size() == 1
                && MessageDigest.isEqual(
                        ("Bearer " + secret).getBytes(StandardCharsets.US_ASCII),
                        values.getFirst().getBytes(StandardCharsets.US_ASCII));
    }

    public CompletableFuture<String> result() {
        return result;
    }

    public static Path directory() {
        return Path.of(System.getProperty("user.home"), ".sonimbus", "google-callbacks");
    }

    @Override
    public void close() {
        result.cancel(false);
        server.stop(0);
        try {
            Files.deleteIfExists(endpoint);
        } catch (IOException failure) {
            mx.uv.sonimbus.desktop.data.logging.AuthLog.unexpectedFailure(
                    "google-channel-cleanup", failure);
        }
    }
}
