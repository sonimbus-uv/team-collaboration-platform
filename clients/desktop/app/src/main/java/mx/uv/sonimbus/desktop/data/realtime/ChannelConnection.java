package mx.uv.sonimbus.desktop.data.realtime;

import com.fasterxml.jackson.core.JsonProcessingException;
import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ArrayNode;
import java.net.URI;
import java.net.http.HttpClient;
import java.net.http.WebSocket;
import java.net.http.WebSocketHandshakeException;
import java.nio.charset.StandardCharsets;
import java.time.Duration;
import java.util.Base64;
import java.util.Map;
import java.util.Objects;
import java.util.Optional;
import java.util.concurrent.CompletableFuture;
import java.util.concurrent.CompletionException;
import java.util.concurrent.CompletionStage;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.Executors;
import java.util.concurrent.ScheduledExecutorService;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.core.Message;
import mx.uv.sonimbus.desktop.data.session.Session;

public final class ChannelConnection implements AutoCloseable {

    public interface Listener {
        void onJoined();

        void onMessage(Message message);

        void onDisconnected();

        void onAccessDenied();
    }

    public static final class SendException extends Exception {
        private final String type;

        public SendException(String type) {
            super("message.create: " + type);
            this.type = type;
        }

        public String type() {
            return type;
        }
    }

    static final String AUTH_TOKEN_PREFIX = "base64url.bearer.phx.";
    static final Duration HEARTBEAT = Duration.ofSeconds(30);
    private static final Duration MAX_BACKOFF = Duration.ofSeconds(10);
    private static final String DISCONNECTED = "disconnected";

    private final HttpClient http;
    private final URI socketUrl;
    private final Session session;
    private final String topic;
    private final Listener listener;
    private final ObjectMapper json;
    private final ScheduledExecutorService scheduler = Executors.newSingleThreadScheduledExecutor(task -> {
        Thread thread = new Thread(task, "sonimbus-realtime");
        thread.setDaemon(true);
        return thread;
    });
    private final AtomicInteger refs = new AtomicInteger();
    private final Map<String, CompletableFuture<Message>> pending = new ConcurrentHashMap<>();

    private WebSocket socket;
    private CompletableFuture<?> outgoing = CompletableFuture.completedFuture(null);
    private ScheduledFuture<?> heartbeat;
    private String joinRef;
    private String tokenInUse;
    private boolean joined;
    private boolean closed;
    private boolean tokenRejected;
    private int failures;

    ChannelConnection(HttpClient http, URI socketUrl, Session session, String channelId, Listener listener,
            ObjectMapper json) {
        this.http = Objects.requireNonNull(http);
        this.socketUrl = Objects.requireNonNull(socketUrl);
        this.session = Objects.requireNonNull(session);
        this.topic = "channel:" + Objects.requireNonNull(channelId);
        this.listener = Objects.requireNonNull(listener);
        this.json = Objects.requireNonNull(json);
    }

    public void connect() {
        scheduler.execute(this::open);
    }

    public CompletableFuture<Message> send(String clientMessageId, String content) {
        CompletableFuture<Message> result = new CompletableFuture<>();
        synchronized (this) {
            if (!joined) {
                result.completeExceptionally(new SendException(DISCONNECTED));
                return result;
            }
            String ref = nextRef();
            pending.put(ref, result);
            push(joinRef, ref, topic, "message.create",
                    Map.of("client_message_id", clientMessageId, "content", content));
        }
        return result;
    }

    @Override
    public void close() {
        WebSocket current;
        synchronized (this) {
            closed = true;
            joined = false;
            current = socket;
            socket = null;
        }
        failPending();
        scheduler.shutdownNow();
        if (current != null) {
            current.sendClose(WebSocket.NORMAL_CLOSURE, "").exceptionally(error -> null);
        }
    }

    private void open() {
        boolean renew;
        String previous;
        synchronized (this) {
            renew = tokenRejected && tokenInUse != null;
            previous = tokenInUse;
        }
        String token;
        try {
            token = renew ? session.renewAfterRejection(previous) : session.accessToken();
        } catch (ApiException e) {
            scheduleReconnect();
            return;
        }
        synchronized (this) {
            if (closed) {
                return;
            }
            tokenInUse = token;
            tokenRejected = false;
        }
        http.newWebSocketBuilder()
                .connectTimeout(Duration.ofSeconds(5))
                .subprotocols("phoenix", AUTH_TOKEN_PREFIX + encodeToken(token))
                .buildAsync(socketUrl, new Receiver())
                .whenComplete((ws, error) -> {
                    if (error != null) {
                        Throwable cause = error instanceof CompletionException && error.getCause() != null
                                ? error.getCause() : error;
                        if (cause instanceof WebSocketHandshakeException handshake
                                && handshake.getResponse().statusCode() == 403) {
                            synchronized (this) {
                                tokenRejected = true;
                            }
                        }
                        scheduleReconnect();
                    } else {
                        onOpened(ws);
                    }
                });
    }

    private synchronized void onOpened(WebSocket ws) {
        if (closed) {
            ws.sendClose(WebSocket.NORMAL_CLOSURE, "");
            return;
        }
        socket = ws;
        outgoing = CompletableFuture.completedFuture(null);
        joinRef = nextRef();
        push(joinRef, joinRef, topic, "phx_join", Map.of());
        heartbeat = scheduler.scheduleAtFixedRate(
                () -> push(null, nextRef(), "phoenix", "heartbeat", Map.of()),
                HEARTBEAT.toSeconds(), HEARTBEAT.toSeconds(), TimeUnit.SECONDS);
    }

    private void handle(JsonNode frame) {
        if (!frame.isArray() || frame.size() != 5) {
            return;
        }
        String ref = frame.get(1).isNull() ? null : frame.get(1).asText();
        String frameTopic = frame.get(2).asText();
        String event = frame.get(3).asText();
        JsonNode payload = frame.get(4);
        if (!topic.equals(frameTopic)) {
            return;
        }
        switch (event) {
            case "phx_reply" -> handleReply(ref, payload);
            case "message.created" -> parseMessage(payload.path("payload")).ifPresent(listener::onMessage);
            case "token.expired", "phx_error", "phx_close" -> dropConnection();
            default -> {
            }
        }
    }

    private void handleReply(String ref, JsonNode payload) {
        boolean ok = "ok".equals(payload.path("status").asText());
        JsonNode response = payload.path("response");
        if (ref != null && ref.equals(joinRef)) {
            if (ok) {
                synchronized (this) {
                    joined = true;
                    failures = 0;
                }
                listener.onJoined();
            } else if ("token_expired".equals(response.path("reason").asText())) {
                dropConnection();
            } else {
                close();
                listener.onAccessDenied();
            }
            return;
        }
        CompletableFuture<Message> push = ref == null ? null : pending.remove(ref);
        if (push == null) {
            return;
        }
        if (ok) {
            parseMessage(response.path("message")).ifPresentOrElse(push::complete,
                    () -> push.completeExceptionally(new SendException("error")));
        } else {
            push.completeExceptionally(new SendException(response.path("type").asText("error")));
        }
    }

    private Optional<Message> parseMessage(JsonNode node) {
        try {
            return Optional.of(json.treeToValue(node, Message.class));
        } catch (JsonProcessingException | IllegalArgumentException e) {
            return Optional.empty();
        }
    }

    private void dropConnection() {
        WebSocket current;
        synchronized (this) {
            current = socket;
            socket = null;
        }
        if (current != null) {
            current.abort();
        }
        lost();
    }

    private void lost() {
        boolean notify;
        synchronized (this) {
            if (closed) {
                return;
            }
            notify = joined;
            joined = false;
            if (heartbeat != null) {
                heartbeat.cancel(false);
                heartbeat = null;
            }
        }
        failPending();
        if (notify) {
            listener.onDisconnected();
        }
        scheduleReconnect();
    }

    private synchronized void scheduleReconnect() {
        if (closed || scheduler.isShutdown()) {
            return;
        }
        failures++;
        long delay = Math.min(MAX_BACKOFF.toMillis(), 500L << Math.min(failures, 5));
        scheduler.schedule(this::open, delay, TimeUnit.MILLISECONDS);
    }

    private void failPending() {
        for (String ref : pending.keySet()) {
            CompletableFuture<Message> push = pending.remove(ref);
            if (push != null) {
                push.completeExceptionally(new SendException(DISCONNECTED));
            }
        }
    }

    private synchronized void push(String pushJoinRef, String ref, String pushTopic, String event, Object payload) {
        if (socket == null) {
            return;
        }
        ArrayNode frame = json.createArrayNode();
        frame.add(pushJoinRef).add(ref).add(pushTopic).add(event).add(json.valueToTree(payload));
        WebSocket target = socket;
        String text = frame.toString();
        outgoing = outgoing.handle((ignored, error) -> null)
                .thenCompose(ignored -> target.sendText(text, true));
    }

    private String nextRef() {
        return Integer.toString(refs.incrementAndGet());
    }

    static String encodeToken(String token) {
        return Base64.getEncoder().withoutPadding().encodeToString(token.getBytes(StandardCharsets.UTF_8));
    }

    private final class Receiver implements WebSocket.Listener {
        private final StringBuilder buffer = new StringBuilder();

        @Override
        public CompletionStage<?> onText(WebSocket webSocket, CharSequence data, boolean last) {
            buffer.append(data);
            if (last) {
                String text = buffer.toString();
                buffer.setLength(0);
                try {
                    handle(json.readTree(text));
                } catch (JsonProcessingException e) {
                }
            }
            webSocket.request(1);
            return null;
        }

        @Override
        public CompletionStage<?> onClose(WebSocket webSocket, int statusCode, String reason) {
            if (webSocket == currentSocket()) {
                lost();
            }
            return null;
        }

        @Override
        public void onError(WebSocket webSocket, Throwable error) {
            if (webSocket == currentSocket()) {
                lost();
            }
        }
    }

    private synchronized WebSocket currentSocket() {
        return socket;
    }
}
