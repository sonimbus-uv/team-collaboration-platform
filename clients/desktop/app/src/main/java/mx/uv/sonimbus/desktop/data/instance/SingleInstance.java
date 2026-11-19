package mx.uv.sonimbus.desktop.data.instance;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;

import mx.uv.sonimbus.desktop.data.local.PrivateLocalFile;

import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.nio.channels.FileChannel;
import java.nio.channels.FileLock;
import java.nio.channels.OverlappingFileLockException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.StandardOpenOption;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.time.Duration;
import java.util.Base64;
import java.util.List;
import java.util.Objects;

public final class SingleInstance implements AutoCloseable {
    private final FileChannel lockFile;
    private final FileLock lock;
    private final Path endpoint;
    private final HttpServer server;
    private final String secret;

    private SingleInstance(FileChannel lockFile, FileLock lock, Path endpoint, Runnable activate)
            throws IOException {
        this.lockFile = lockFile;
        this.lock = lock;
        this.endpoint = endpoint;
        byte[] random = new byte[32];
        new SecureRandom().nextBytes(random);
        secret = Base64.getUrlEncoder().withoutPadding().encodeToString(random);
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        try {
            server.createContext("/activate", request -> receive(request, activate));
            Files.deleteIfExists(endpoint);
            PrivateLocalFile.create(endpoint);
            Files.writeString(endpoint, server.getAddress().getPort() + "\n" + secret);
            server.start();
        } catch (IOException | RuntimeException failure) {
            server.stop(0);
            Files.deleteIfExists(endpoint);
            throw failure;
        }
    }

    public static SingleInstance acquire(Path directory, Runnable activate) throws IOException {
        Objects.requireNonNull(activate);
        Files.createDirectories(directory);
        FileChannel lockFile =
                FileChannel.open(
                        directory.resolve("client.lock"),
                        StandardOpenOption.CREATE,
                        StandardOpenOption.WRITE);
        FileLock lock = null;
        try {
            lock = tryLock(lockFile);
            Path endpoint = directory.resolve("client.endpoint");
            if (lock == null) {
                lockFile.close();
                activateExisting(endpoint);
                return null;
            }
            return new SingleInstance(lockFile, lock, endpoint, activate);
        } catch (IOException | RuntimeException failure) {
            if (lock != null) {
                lock.release();
            }
            lockFile.close();
            throw failure;
        }
    }

    private static FileLock tryLock(FileChannel file) throws IOException {
        try {
            return file.tryLock();
        } catch (OverlappingFileLockException alreadyRunning) {
            return null;
        }
    }

    private void receive(HttpExchange request, Runnable activate) throws IOException {
        try {
            List<String> headers = request.getRequestHeaders().get("Authorization");
            if (!"POST".equals(request.getRequestMethod())
                    || !"/activate".equals(request.getRequestURI().toString())
                    || headers == null
                    || headers.size() != 1
                    || !MessageDigest.isEqual(
                            ("Bearer " + secret).getBytes(StandardCharsets.US_ASCII),
                            headers.getFirst().getBytes(StandardCharsets.US_ASCII))) {
                request.sendResponseHeaders(403, -1);
                return;
            }
            activate.run();
            request.sendResponseHeaders(204, -1);
        } finally {
            request.close();
        }
    }

    private static void activateExisting(Path endpoint) throws IOException {
        try (HttpClient client =
                HttpClient.newBuilder().connectTimeout(Duration.ofSeconds(1)).build()) {
            for (int attempt = 0; attempt < 10; attempt++) {
                try {
                    if (Files.isRegularFile(endpoint)
                            && !Files.isSymbolicLink(endpoint)
                            && Files.size(endpoint) <= 128
                            && sendActivation(client, endpoint)) {
                        return;
                    }
                } catch (IOException startingUp) {
                    if (attempt == 9) {
                        throw startingUp;
                    }
                }
                Thread.sleep(100);
            }
            throw new IOException("The existing client could not be activated");
        } catch (InterruptedException interrupted) {
            Thread.currentThread().interrupt();
            throw new IOException("Client activation interrupted");
        }
    }

    private static boolean sendActivation(HttpClient client, Path endpoint)
            throws IOException, InterruptedException {
        String[] values = Files.readString(endpoint).split("\n", -1);
        if (values.length != 2 || !values[1].matches("[A-Za-z0-9_-]{43}")) {
            return false;
        }
        int port;
        try {
            port = Integer.parseInt(values[0]);
        } catch (NumberFormatException malformed) {
            return false;
        }
        if (port < 1 || port > 65535) {
            return false;
        }
        HttpRequest request =
                HttpRequest.newBuilder(URI.create("http://127.0.0.1:" + port + "/activate"))
                        .timeout(Duration.ofSeconds(1))
                        .header("Authorization", "Bearer " + values[1])
                        .POST(HttpRequest.BodyPublishers.noBody())
                        .build();
        return client.send(request, HttpResponse.BodyHandlers.discarding()).statusCode() == 204;
    }

    @Override
    public void close() throws IOException {
        server.stop(0);
        try {
            Files.deleteIfExists(endpoint);
        } finally {
            try {
                lock.release();
            } finally {
                lockFile.close();
            }
        }
    }
}
