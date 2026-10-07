package mx.uv.sonimbus.desktop.ui.navigation;

import static org.junit.jupiter.api.Assertions.assertNotNull;

import org.junit.jupiter.api.Test;

class ViewTest {

    @Test
    void everyViewHasItsFxml() {
        for (View view : View.values()) {
            assertNotNull(View.class.getResource(ViewManager.UI_ROOT + view.fxml()), view.name());
        }
    }
}
