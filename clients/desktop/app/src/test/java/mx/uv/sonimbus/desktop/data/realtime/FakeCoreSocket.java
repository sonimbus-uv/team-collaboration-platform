package mx.uv.sonimbus.desktop.data.realtime;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import java.io.ByteArrayOutputStream;
import java.io.DataInputStream;
import java.io.IOException;
import java.io.OutputStream;
import java.net.InetAddress;
import java.net.ServerSocket;
import java.net.Socket;
import java.net.URI;
import java.nio.charset.StandardCharsets;
import java.security.MessageDigest;
import java.util.Base64;
import java.util.List;
import java.util.concurrent.BlockingQueue;
import java.util.concurrent.CopyOnWriteArrayList;
import java.util.concurrent.LinkedBlockingQueue;
import java.util.concurrent.TimeUnit;

final class FakeCoreSocket implements AutoCloseable {

    private static final String GUID = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

    private final ServerSocket server;
    private final ObjectMapper json = new ObjectMapper();
    private final List<String> subprotocols = new CopyOnWriteArrayList<>();
    private final BlockingQueue<JsonNode> frames = new LinkedBlockingQueue<>();
    private final BlockingQueue<Socket> connections = new LinkedBlockingQueue<>();
    private volatile Socket current;
    private volatile boolean autoJoin = true;

    FakeCoreSocket() throws IOException {
        server = new ServerSocket(0, 10, InetAddress.getLoopbackAddress());
        Thread acceptor = new Thread(this::acceptLoop, "fake-core-socket");
        acceptor.setDaemon(true);
        acceptor.start();
    }

    URI url() {
        return URI.create("ws://127.0.0.1:" + server.getLocalPort() + "/socket/websocket");
    }

    List<String> subprotocols() {
        return subprotocols;
    }

    void autoJoin(boolean enabled) {
        autoJoin = enabled;
    }

    JsonNode nextFrame(String event) throws InterruptedException {
        long deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(10);
        while (System.nanoTime() < deadline) {
            JsonNode frame = frames.poll(100, TimeUnit.MILLISECONDS);
            if (frame != null && event.equals(frame.get(3).asText())) {
                return frame;
            }
        }
        throw new AssertionError("No frame " + event);
    }

    void awaitConnection() throws InterruptedException {
        if (connections.poll(10, TimeUnit.SECONDS) == null) {
            throw new AssertionError("No connection");
        }
    }

    void push(String event, String payload) throws IOException {
        sendText("[null,null,\"channel:c1\",\"" + event + "\"," + payload + "]");
    }

    void dropConnection() throws IOException {
        Socket socket = current;
        if (socket != null) {
            socket.close();
        }
    }

    @Override
    public void close() throws IOException {
        dropConnection();
        server.close();
    }

    private void acceptLoop() {
        while (!server.isClosed()) {
            try {
                Socket socket = server.accept();
                Thread worker = new Thread(() -> serve(socket), "fake-core-connection");
                worker.setDaemon(true);
                worker.start();
            } catch (IOException e) {
                return;
            }
        }
    }

    private void serve(Socket socket) {
        try {
            DataInputStream in = new DataInputStream(socket.getInputStream());
            String key = null;
            StringBuilder line = new StringBuilder();
            while (true) {
                int b = in.read();
                if (b < 0) {
                    return;
                }
                if (b == '\n') {
                    String text = line.toString().strip();
                    line.setLength(0);
                    if (text.isEmpty()) {
                        break;
                    }
                    String lower = text.toLowerCase();
                    if (lower.startsWith("sec-websocket-key:")) {
                        key = text.substring(text.indexOf(':') + 1).strip();
                    } else if (lower.startsWith("sec-websocket-protocol:")) {
                        subprotocols.add(text.substring(text.indexOf(':') + 1).strip());
                    }
                } else {
                    line.append((char) b);
                }
            }
            String accept = Base64.getEncoder().encodeToString(
                    MessageDigest.getInstance("SHA-1").digest((key + GUID).getBytes(StandardCharsets.UTF_8)));
            OutputStream out = socket.getOutputStream();
            out.write(("HTTP/1.1 101 Switching Protocols\r\nUpgrade: websocket\r\nConnection: Upgrade\r\n"
                    + "Sec-WebSocket-Accept: " + accept + "\r\nSec-WebSocket-Protocol: phoenix\r\n\r\n")
                    .getBytes(StandardCharsets.UTF_8));
            out.flush();
            current = socket;
            connections.add(socket);
            readFrames(socket, in);
        } catch (Exception e) {
            return;
        }
    }

    private void readFrames(Socket socket, DataInputStream in) throws IOException {
        while (true) {
            int first = in.read();
            if (first < 0) {
                return;
            }
            int opcode = first & 0x0f;
            int second = in.read();
            long length = second & 0x7f;
            if (length == 126) {
                length = in.readUnsignedShort();
            } else if (length == 127) {
                length = in.readLong();
            }
            byte[] mask = new byte[4];
            in.readFully(mask);
            byte[] payload = new byte[(int) length];
            in.readFully(payload);
            for (int i = 0; i < payload.length; i++) {
                payload[i] ^= mask[i % 4];
            }
            if (opcode == 8) {
                return;
            }
            if (opcode != 1) {
                continue;
            }
            JsonNode frame = json.readTree(new String(payload, StandardCharsets.UTF_8));
            frames.add(frame);
            if (autoJoin && "phx_join".equals(frame.get(3).asText())) {
                String ref = frame.get(1).asText();
                sendTo(socket, "[\"" + ref + "\",\"" + ref + "\",\"channel:c1\",\"phx_reply\","
                        + "{\"status\":\"ok\",\"response\":{}}]");
            }
        }
    }

    private void sendText(String text) throws IOException {
        sendTo(current, text);
    }

    private static void sendTo(Socket socket, String text) throws IOException {
        byte[] payload = text.getBytes(StandardCharsets.UTF_8);
        ByteArrayOutputStream frame = new ByteArrayOutputStream();
        frame.write(0x81);
        if (payload.length < 126) {
            frame.write(payload.length);
        } else {
            frame.write(126);
            frame.write(payload.length >> 8);
            frame.write(payload.length & 0xff);
        }
        frame.write(payload);
        synchronized (socket) {
            socket.getOutputStream().write(frame.toByteArray());
            socket.getOutputStream().flush();
        }
    }
}
