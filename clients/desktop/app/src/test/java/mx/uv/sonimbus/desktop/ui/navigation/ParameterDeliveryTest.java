package mx.uv.sonimbus.desktop.ui.navigation;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNull;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

class ParameterDeliveryTest {

    record Email(String value) {
    }

    static class Target implements Parameterized<Email> {
        Email received;

        @Override
        public Class<Email> parametersType() {
            return Email.class;
        }

        @Override
        public void setParameters(Email parameters) {
            received = parameters;
        }
    }

    @Test
    void parametersReachTheController() {
        Target target = new Target();

        ViewManager.deliverParameters(View.VERIFY_ACCOUNT, target, new Email("ana@uv.mx"));

        assertEquals(new Email("ana@uv.mx"), target.received);
    }

    @Test
    void withoutParametersNothingIsCalled() {
        Target target = new Target();

        ViewManager.deliverParameters(View.VERIFY_ACCOUNT, target, null);
        ViewManager.deliverParameters(View.LOGIN, new Object(), null);

        assertNull(target.received);
    }

    @Test
    void wrongTypeFailsClearly() {
        IllegalArgumentException e = assertThrows(IllegalArgumentException.class,
                () -> ViewManager.deliverParameters(View.VERIFY_ACCOUNT, new Target(), "ana@uv.mx"));

        assertEquals("La vista VERIFY_ACCOUNT espera Email, no String", e.getMessage());
    }

    @Test
    void controllerWithoutParametersRejectsThem() {
        IllegalArgumentException e = assertThrows(IllegalArgumentException.class,
                () -> ViewManager.deliverParameters(View.LOGIN, new Object(), new Email("x")));

        assertEquals("La vista LOGIN no recibe parámetros", e.getMessage());
    }
}
