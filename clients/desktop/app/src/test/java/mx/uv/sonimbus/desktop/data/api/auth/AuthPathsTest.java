package mx.uv.sonimbus.desktop.data.api.auth;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class AuthPathsTest {
    @Test
    void pathsShareOnePrefix() {
        assertEquals(AuthPaths.PREFIX + "/auth/register", AuthPaths.REGISTER);
        assertEquals(AuthPaths.PREFIX + "/auth/verify-email", AuthPaths.VERIFY_EMAIL);
        assertEquals(AuthPaths.PREFIX + "/auth/resend-verification", AuthPaths.RESEND_VERIFICATION);
    }
}
