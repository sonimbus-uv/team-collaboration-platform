package mx.uv.sonimbus.desktop.data.api;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.net.URI;
import org.junit.jupiter.api.Test;

class ApiConfigTest {

    @Test
    void propertyWinsOverEnvironment() {
        assertEquals(URI.create("http://a:1"), ApiConfig.resolve(" http://a:1 ", "http://b:2"));
    }

    @Test
    void environmentIsUsedWithoutProperty() {
        assertEquals(URI.create("http://b:2"), ApiConfig.resolve(" ", "http://b:2"));
    }

    @Test
    void localServiceIsTheDefault() {
        assertEquals(URI.create(ApiConfig.DEFAULT_AUTH_URL), ApiConfig.resolve(null, null));
    }
}
