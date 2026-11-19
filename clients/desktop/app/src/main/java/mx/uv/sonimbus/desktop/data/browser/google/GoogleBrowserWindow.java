package mx.uv.sonimbus.desktop.data.browser.google;

import com.sun.net.httpserver.HttpExchange;
import com.sun.net.httpserver.HttpServer;

import java.io.IOException;
import java.net.InetSocketAddress;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.security.SecureRandom;
import java.util.Base64;
import java.util.List;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;

public final class GoogleBrowserWindow implements AutoCloseable {
    private static final String PAGE = "/mx/uv/sonimbus/desktop/ui/auth/google/browser-window.html";
    private final HttpServer server;
    private final CompletableFuture<?> completion;
    private final String browserKey;
    private final byte[] page;
    private final AtomicBoolean closing = new AtomicBoolean();
    private final AtomicBoolean stopped = new AtomicBoolean();
    private volatile boolean opened;

    public GoogleBrowserWindow(URI authorization, CompletableFuture<?> completion)
            throws IOException {
        this.completion = completion;
        byte[] random = new byte[32];
        new SecureRandom().nextBytes(random);
        browserKey = Base64.getUrlEncoder().withoutPadding().encodeToString(random);
        page = render(authorization);
        server = HttpServer.create(new InetSocketAddress("127.0.0.1", 0), 0);
        server.createContext("/browser", this::servePage);
        server.createContext("/status", this::serveStatus);
        server.start();
    }

    private byte[] render(URI authorization) throws IOException {
        try (var resource = GoogleBrowserWindow.class.getResourceAsStream(PAGE)) {
            if (resource == null) {
                throw new IOException("Google browser page unavailable");
            }
            return new String(resource.readAllBytes(), StandardCharsets.UTF_8)
                    .replace("__AUTHORIZATION_URL__", jsonString(authorization.toASCIIString()))
                    .replace("__BROWSER_KEY__", browserKey)
                    .getBytes(StandardCharsets.UTF_8);
        }
    }

    private static String jsonString(String value) {
        return "\""
                + value.replace("\\", "\\\\")
                        .replace("\"", "\\\"")
                        .replace("<", "\\u003c")
                        .replace(">", "\\u003e")
                        .replace("&", "\\u0026")
                + "\"";
    }

    public URI launchUri() {
        return URI.create(
                "http://127.0.0.1:" + server.getAddress().getPort() + "/browser?key=" + browserKey);
    }

    private void servePage(HttpExchange request) throws IOException {
        try {
            if (!"GET".equals(request.getRequestMethod())
                    || !"/browser".equals(request.getRequestURI().getRawPath())
                    || !("key=" + browserKey).equals(request.getRequestURI().getRawQuery())) {
                request.sendResponseHeaders(403, -1);
                return;
            }
            opened = true;
            request.getResponseHeaders().set("Content-Type", "text/html; charset=utf-8");
            request.getResponseHeaders()
                    .set(
                            "Content-Security-Policy",
                            "default-src 'none'; script-src 'nonce-"
                                    + browserKey
                                    + "'; style-src 'unsafe-inline'; connect-src 'self';"
                                    + " frame-ancestors 'none'; base-uri 'none'");
            request.getResponseHeaders().set("Referrer-Policy", "no-referrer");
            request.getResponseHeaders().set("X-Content-Type-Options", "nosniff");
            request.getResponseHeaders().set("Cache-Control", "no-store");
            request.sendResponseHeaders(200, page.length);
            request.getResponseBody().write(page);
        } finally {
            request.close();
        }
    }

    private void serveStatus(HttpExchange request) throws IOException {
        try {
            List<String> values = request.getRequestHeaders().get("X-Sonimbus-Browser");
            if (!"POST".equals(request.getRequestMethod())
                    || !"/status".equals(request.getRequestURI().toString())
                    || values == null
                    || values.size() != 1
                    || !MessageDigest.isEqual(
                            browserKey.getBytes(StandardCharsets.US_ASCII),
                            values.getFirst().getBytes(StandardCharsets.US_ASCII))) {
                request.sendResponseHeaders(403, -1);
                return;
            }
            boolean received = completion.isDone();
            byte[] response =
                    ("{\"received\":" + received + "}").getBytes(StandardCharsets.US_ASCII);
            request.getResponseHeaders().set("Content-Type", "application/json");
            request.getResponseHeaders().set("Cache-Control", "no-store");
            request.sendResponseHeaders(200, response.length);
            request.getResponseBody().write(response);
            request.close();
            if (received && closing.get()) {
                CompletableFuture.delayedExecutor(1, TimeUnit.SECONDS).execute(this::stopServer);
            }
        } finally {
            request.close();
        }
    }

    private void stopServer() {
        if (stopped.compareAndSet(false, true)) {
            server.stop(0);
        }
    }

    @Override
    public void close() {
        if (!closing.compareAndSet(false, true)) {
            return;
        }
        if (!opened) {
            stopServer();
            return;
        }
        // Keep the local page reachable briefly so it can acknowledge and close its popup.
        CompletableFuture.delayedExecutor(10, TimeUnit.SECONDS).execute(this::stopServer);
    }
}
