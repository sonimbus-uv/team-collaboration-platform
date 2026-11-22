package mx.uv.sonimbus.desktop.data.realtime;

import com.fasterxml.jackson.databind.DeserializationFeature;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import java.net.URI;
import java.net.http.HttpClient;
import mx.uv.sonimbus.desktop.data.api.ApiConfig;
import mx.uv.sonimbus.desktop.data.session.Session;

public class RealtimeClient {

    private final HttpClient http;
    private final URI socketUrl;
    private final Session session;
    private final ObjectMapper json = new ObjectMapper()
            .setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE)
            .configure(DeserializationFeature.FAIL_ON_UNKNOWN_PROPERTIES, false);

    public RealtimeClient(ApiConfig config, Session session) {
        this.http = HttpClient.newBuilder().connectTimeout(config.connectTimeout()).build();
        this.socketUrl = config.coreSocketUrl();
        this.session = session;
    }

    public ChannelConnection channel(String channelId, ChannelConnection.Listener listener) {
        return new ChannelConnection(http, socketUrl, session, channelId, listener, json);
    }
}
