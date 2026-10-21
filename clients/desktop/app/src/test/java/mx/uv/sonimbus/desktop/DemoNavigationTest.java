package mx.uv.sonimbus.desktop;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.EnumSet;
import java.util.List;
import java.util.Optional;
import java.util.Set;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.function.IntSupplier;
import mx.uv.sonimbus.desktop.DemoNavigation.Placement;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import org.junit.jupiter.api.Test;

class DemoNavigationTest {

    private static final IntSupplier UNUSED = () -> {
        throw new AssertionError("solo «siguiente» usa el contador");
    };

    @Test
    void withoutArgumentsThereIsNoDemo() {
        assertEquals(Optional.empty(), DemoNavigation.requestedView(List.of(), UNUSED));
        assertEquals(Optional.empty(), DemoNavigation.requestedView(List.of(" "), UNUSED));
    }

    @Test
    void nextWalksAllViewsAndWrapsAround() {
        AtomicInteger counter = new AtomicInteger();
        View[] views = View.values();
        for (int i = 0; i <= views.length; i++) {
            View view = DemoNavigation.requestedView(List.of("Siguiente"), counter::getAndIncrement).orElseThrow();
            assertEquals(views[i % views.length], view);
        }
    }

    @Test
    void numbersStartAtOne() {
        assertEquals(Optional.of(View.LOGIN), DemoNavigation.requestedView(List.of("1"), UNUSED));
        assertEquals(Optional.of(View.VERIFY_ACCOUNT), DemoNavigation.requestedView(List.of(" 3 "), UNUSED));
        assertEquals(Optional.of(View.PROFILE), DemoNavigation.requestedView(List.of("16"), UNUSED));
        assertEquals(Optional.empty(), DemoNavigation.requestedView(List.of("0"), UNUSED));
        assertEquals(Optional.empty(), DemoNavigation.requestedView(List.of("17"), UNUSED));
    }

    @Test
    void namesIgnoreCaseAndAcceptHyphens() {
        assertEquals(Optional.of(View.MAIN), DemoNavigation.requestedView(List.of("main"), UNUSED));
        assertEquals(Optional.of(View.VERIFY_ACCOUNT), DemoNavigation.requestedView(List.of("verify-account"), UNUSED));
        assertEquals(Optional.of(View.GROUP_SETTINGS), DemoNavigation.requestedView(List.of("GROUP_SETTINGS"), UNUSED));
        assertEquals(Optional.empty(), DemoNavigation.requestedView(List.of("inexistente"), UNUSED));
    }

    @Test
    void everyViewHasOnePlacement() {
        Set<View> content = EnumSet.complementOf(EnumSet.copyOf(DemoNavigation.FULL_WINDOW));
        content.removeAll(DemoNavigation.DIALOGS);
        for (View view : View.values()) {
            Placement expected = DemoNavigation.FULL_WINDOW.contains(view) ? Placement.FULL_WINDOW
                    : DemoNavigation.DIALOGS.contains(view) ? Placement.DIALOG : Placement.CONTENT;
            assertEquals(expected, DemoNavigation.placementOf(view), view.name());
        }
        assertEquals(EnumSet.of(View.TEXT_CHANNEL, View.VOICE_CHANNEL, View.SHARED_FILES, View.GROUP_SETTINGS,
                View.PROJECTS, View.REPORTS, View.NOTIFICATIONS, View.PROFILE), content);
    }

    @Test
    void titlesAndUsage() {
        assertEquals("Sonimbus · 3 de 16 · VERIFY_ACCOUNT", DemoNavigation.windowTitle(View.VERIFY_ACCOUNT));
        assertEquals("Crear grupo", DemoNavigation.dialogTitle(View.CREATE_GROUP));
        String usage = DemoNavigation.usage("xyz");
        assertTrue(usage.startsWith("Vista desconocida: xyz. Opciones:"));
        assertTrue(usage.contains("16  PROFILE"));
    }
}
