package mx.uv.sonimbus.desktop.ui.auth.registration;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class VerificationCooldownTest {
    @Test
    void cooldownCountdownText() {

        assertEquals("Reenviar en 42 s", VerifyAccountController.cooldownText(42));

        assertEquals(60, VerifyAccountController.RESEND_COOLDOWN_SECONDS);
    }
}
