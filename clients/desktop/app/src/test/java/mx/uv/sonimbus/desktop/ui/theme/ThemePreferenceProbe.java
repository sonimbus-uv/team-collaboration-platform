package mx.uv.sonimbus.desktop.ui.theme;

import java.util.prefs.Preferences;

public final class ThemePreferenceProbe {

    private ThemePreferenceProbe() {}

    public static void main(String[] arguments) {
        System.out.println(Preferences.userRoot().node(arguments[0]).get("appearance", "LIGHT"));
    }
}
