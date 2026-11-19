package mx.uv.sonimbus.desktop.data.browser.google;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import org.junit.jupiter.api.Test;

import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.HttpRequest;
import java.net.http.HttpResponse;
import java.util.concurrent.CompletableFuture;

class GoogleBrowserWindowTest {
    private static final URI AUTHORIZATION =
            URI.create(
                    "https://auth.invalid/api/v1/auth/google/authorize?ticket=public-browser-ticket");

    @Test
    void localPageUsesAnOwnedPopupWithRestrictedContentPolicy() throws Exception {
        try (var window = new GoogleBrowserWindow(AUTHORIZATION, new CompletableFuture<>());
                var client = HttpClient.newHttpClient()) {
            var response =
                    client.send(
                            HttpRequest.newBuilder(window.launchUri()).build(),
                            HttpResponse.BodyHandlers.ofString());
            assertEquals(200, response.statusCode());
            assertTrue(response.body().contains("window.open("));
            assertTrue(response.body().contains("authenticationWindow.close()"));
            assertTrue(response.body().contains("Continuar con Google"));
            assertFalse(response.body().contains("__AUTHORIZATION_URL__"));
            assertEquals("no-store", response.headers().firstValue("Cache-Control").orElseThrow());
            assertTrue(
                    response.headers()
                            .firstValue("Content-Security-Policy")
                            .orElseThrow()
                            .contains("frame-ancestors 'none'"));
        }
    }

    @Test
    void statusRemainsAvailableBrieflyAfterNativeReturnAndResourceClose() throws Exception {
        CompletableFuture<String> completion = new CompletableFuture<>();
        try (var window = new GoogleBrowserWindow(AUTHORIZATION, completion);
                var client = HttpClient.newHttpClient()) {
            client.send(
                    HttpRequest.newBuilder(window.launchUri()).build(),
                    HttpResponse.BodyHandlers.discarding());
            assertTrue(status(client, window).contains("false"));
            completion.complete("received");
            window.close();
            assertTrue(status(client, window).contains("true"));
        }
    }

    @Test
    void cancellationAlsoClosesTheOwnedAuthenticationWindow() throws Exception {
        CompletableFuture<String> completion = new CompletableFuture<>();
        try (var window = new GoogleBrowserWindow(AUTHORIZATION, completion);
                var client = HttpClient.newHttpClient()) {
            completion.cancel(false);
            assertTrue(status(client, window).contains("true"));
        }
    }

    @Test
    void otherWebsitesCannotReadThePageOrPollTheAttempt() throws Exception {
        try (var window = new GoogleBrowserWindow(AUTHORIZATION, new CompletableFuture<>());
                var client = HttpClient.newHttpClient()) {
            URI noKey = window.launchUri().resolve("/browser");
            assertEquals(
                    403,
                    client.send(
                                    HttpRequest.newBuilder(noKey).build(),
                                    HttpResponse.BodyHandlers.discarding())
                            .statusCode());
            assertEquals(
                    403,
                    client.send(
                                    HttpRequest.newBuilder(window.launchUri().resolve("/status"))
                                            .POST(HttpRequest.BodyPublishers.noBody())
                                            .build(),
                                    HttpResponse.BodyHandlers.discarding())
                            .statusCode());
        }
    }

    private String status(HttpClient client, GoogleBrowserWindow window) throws Exception {
        String key = window.launchUri().getRawQuery().substring(4);
        return client.send(
                        HttpRequest.newBuilder(window.launchUri().resolve("/status"))
                                .header("X-Sonimbus-Browser", key)
                                .POST(HttpRequest.BodyPublishers.noBody())
                                .build(),
                        HttpResponse.BodyHandlers.ofString())
                .body();
    }
}
