package mx.uv.sonimbus.desktop.ui.common;

import java.util.Locale;

public final class Initials {

    private Initials() {
    }

    public static String of(String name) {
        if (name == null || name.isBlank()) {
            return "?";
        }
        StringBuilder initials = new StringBuilder();
        for (String word : name.strip().split("[\\s_-]+")) {
            if (!word.isEmpty() && initials.length() < 2) {
                initials.appendCodePoint(word.codePointAt(0));
            }
        }
        return initials.toString().toUpperCase(Locale.ROOT);
    }
}
