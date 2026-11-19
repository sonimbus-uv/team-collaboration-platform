package mx.uv.sonimbus.desktop.ui.groups;

import static org.junit.jupiter.api.Assertions.assertEquals;

import org.junit.jupiter.api.Test;

class JoinGroupControllerTest {

    @Test
    void codeIsTakenAloneOrFromTheEndOfALink() {
        assertEquals("Ab12-x", JoinGroupController.extractCode("  Ab12-x "));
        assertEquals("Ab12-x", JoinGroupController.extractCode("https://sonimbus.example/invite/Ab12-x/"));
        assertEquals("", JoinGroupController.extractCode(null));
    }
}
