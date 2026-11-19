package mx.uv.sonimbus.desktop.data.realtime;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.nio.charset.StandardCharsets;
import java.util.Base64;
import org.junit.jupiter.api.Test;

class ChannelConnectionTest {

    @Test
    void tokenIsEncodedLikeThePhoenixClient() {
        // btoa("ab?") = "YWI/" en Base64 estándar; Phoenix lo decodifica con padding: false.
        assertEquals("YWI/", ChannelConnection.encodeToken("ab?"));
        String jwt = "eyJhbGciOiJFZERTQSJ9.eyJzdWIiOiJ1MSJ9.c2ln";
        String encoded = ChannelConnection.encodeToken(jwt);
        assertEquals(jwt, new String(Base64.getDecoder().decode(encoded), StandardCharsets.UTF_8));
        assertEquals(-1, encoded.indexOf('='));
    }
}
