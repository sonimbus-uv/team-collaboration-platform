package mx.uv.sonimbus.desktop.ui.navigation;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertInstanceOf;
import static org.junit.jupiter.api.Assertions.assertSame;
import static org.junit.jupiter.api.Assertions.assertThrows;

import org.junit.jupiter.api.Test;

class ControllerFactoryTest {

    public static class Skeleton {
    }

    public static class NeedsService {
        final StringBuilder service;

        public NeedsService(StringBuilder service) {
            this.service = service;
        }
    }

    public static class TwoConstructors {
        public TwoConstructors() {
        }

        public TwoConstructors(StringBuilder service) {
        }
    }

    public static class Failing {
        public Failing() {
            throw new IllegalStateException("boom");
        }
    }

    private final ControllerFactory factory = new ControllerFactory();

    @Test
    void skeletonsWithoutDependenciesStillWork() {
        assertInstanceOf(Skeleton.class, factory.call(Skeleton.class));
    }

    @Test
    void injectsRegisteredServicesByType() {
        StringBuilder service = new StringBuilder();
        factory.register(StringBuilder.class, service);

        NeedsService controller = (NeedsService) factory.call(NeedsService.class);

        assertSame(service, controller.service);
    }

    @Test
    void missingServiceIsNamed() {
        IllegalStateException e = assertThrows(IllegalStateException.class, () -> factory.call(NeedsService.class));

        assertEquals("NeedsService necesita StringBuilder, que no está registrado", e.getMessage());
    }

    @Test
    void ambiguousConstructorsAreRejected() {
        assertThrows(IllegalStateException.class, () -> factory.call(TwoConstructors.class));
    }

    @Test
    void constructorFailureKeepsItsCause() {
        IllegalStateException e = assertThrows(IllegalStateException.class, () -> factory.call(Failing.class));

        assertEquals("boom", e.getCause().getMessage());
    }
}
