package mx.uv.sonimbus.desktop.data.api;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertThrows;

import java.net.URI;
import java.time.Duration;
import org.junit.jupiter.api.Test;

class ApiConfigTest {

    @Test
    void propertyWinsOverEnvironment() {
        assertEquals(URI.create("http://a:1"), ApiConfig.resolveAuthUrl(" http://a:1 ", "http://b:2"));
    }

    @Test
    void environmentIsUsedWithoutProperty() {
        assertEquals(URI.create("http://b:2"), ApiConfig.resolveAuthUrl(" ", "http://b:2"));
    }

    @Test
    void vagrantVmIsTheDefault() {
        assertEquals(URI.create("http://192.168.33.30:3001"), ApiConfig.resolveAuthUrl(null, null));
    }

    @Test
    void trailingSlashIsRemoved() {
        assertEquals(URI.create("http://a:1"), ApiConfig.resolveAuthUrl("http://a:1//", null));
    }

    @Test
    void timeoutsFollowRnfRen01() {
        ApiConfig config = ApiConfig.withAuthUrl(URI.create("http://a:1"));

        assertEquals(Duration.ofSeconds(5), config.connectTimeout());
        assertEquals(Duration.ofSeconds(10), config.requestTimeout());
    }

    @Test
    void rejectsUrlsThatAreNotHttp() {
        assertThrows(IllegalArgumentException.class, () -> ApiConfig.withAuthUrl(URI.create("ftp://a:1")));
        assertThrows(IllegalArgumentException.class, () -> ApiConfig.withAuthUrl(URI.create("192.168.33.30:3001")));
    }

    @Test
    void rejectsCredentialsWithoutEchoingThem() {
        IllegalArgumentException e = assertThrows(IllegalArgumentException.class,
                () -> ApiConfig.withAuthUrl(URI.create("http://admin:s3cret@a:1")));

        assertFalse(e.getMessage().contains("s3cret"));
    }

    @Test
    void malformedUrlNamesItsSource() {
        IllegalArgumentException e = assertThrows(IllegalArgumentException.class,
                () -> ApiConfig.resolveAuthUrl(null, "http://a b"));

        assertEquals("La dirección de SONIMBUS_AUTH_URL no es una URL válida", e.getMessage());
    }

    @Test
    void pathsShareOnePrefix() {
        assertEquals(AuthPaths.PREFIX + "/auth/register", AuthPaths.REGISTER);
        assertEquals(AuthPaths.PREFIX + "/auth/verify-email", AuthPaths.VERIFY_EMAIL);
        assertEquals(AuthPaths.PREFIX + "/auth/resend-verification", AuthPaths.RESEND_VERIFICATION);
    }
}
