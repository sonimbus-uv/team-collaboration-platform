package mx.uv.sonimbus.desktop.ui.theme;

import javafx.beans.property.ObjectProperty;
import javafx.beans.property.SimpleObjectProperty;
import javafx.collections.ListChangeListener;
import javafx.collections.WeakListChangeListener;
import javafx.scene.Node;
import javafx.scene.Parent;
import javafx.scene.control.PopupControl;
import javafx.stage.PopupWindow;
import javafx.stage.Window;

import java.util.ArrayList;
import java.util.Map;
import java.util.Objects;
import java.util.WeakHashMap;
import java.util.logging.Logger;
import java.util.prefs.BackingStoreException;
import java.util.prefs.Preferences;

public final class ThemeManager {

    private static final String PREFERENCE_KEY = "appearance";
    private static final String STYLE_OWNER = ThemeManager.class.getName();
    private static final String STYLESHEET =
            Objects.requireNonNull(
                            ThemeManager.class.getResource("/mx/uv/sonimbus/desktop/css/theme.css"))
                    .toExternalForm();
    private final Preferences preferences;
    private final ObjectProperty<Theme> theme;
    private final Map<Parent, Boolean> roots = new WeakHashMap<>();
    private final Map<PopupControl, Boolean> popups = new WeakHashMap<>();
    private final ListChangeListener<Window> windowsListener = this::windowsChanged;

    public ThemeManager() {
        this(Preferences.userNodeForPackage(ThemeManager.class));
    }

    public ThemeManager(Preferences preferences) {
        this.preferences = Objects.requireNonNull(preferences);
        theme = new SimpleObjectProperty<>(readTheme());
        theme.addListener(
                (observable, previous, current) -> {
                    for (Parent root : new ArrayList<>(roots.keySet())) {
                        applyTheme(root);
                    }
                    saveTheme(current);
                });
        Window.getWindows().addListener(new WeakListChangeListener<>(windowsListener));
    }

    public Theme getTheme() {
        return theme.get();
    }

    public void setTheme(Theme selectedTheme) {
        theme.set(Objects.requireNonNull(selectedTheme));
    }

    public void register(Parent root) {

        if (roots.put(root, true) != null) {
            return;
        }

        root.getProperties().put(STYLE_OWNER, this);

        if (!root.getStylesheets().contains(STYLESHEET)) {
            root.getStylesheets().add(STYLESHEET);
        }

        applyTheme(root);

        for (var node : root.lookupAll(".theme-selector")) {
            if (node instanceof ThemeSelector selector) {
                bindSelector(selector);
            }
        }
    }

    static void installSelector(ThemeSelector selector) {

        for (Node node = selector.getParent(); node != null; node = node.getParent()) {
            if (node.getProperties().get(STYLE_OWNER) instanceof ThemeManager manager) {
                manager.bindSelector(selector);
                return;
            }
        }
    }

    private void bindSelector(ThemeSelector selector) {
        if (selector.getProperties().putIfAbsent(STYLE_OWNER, this) == null) {
            selector.setValue(getTheme());
            selector.valueProperty().bindBidirectional(theme);
        }
    }

    private void applyTheme(Parent root) {
        root.getStyleClass().removeAll("theme-light", "theme-dark");
        root.getStyleClass().add(getTheme().styleClass());
    }

    private Theme readTheme() {

        try {

            return Theme.valueOf(preferences.get(PREFERENCE_KEY, Theme.LIGHT.name()));

        } catch (IllegalArgumentException | SecurityException invalidPreference) {
            return Theme.LIGHT;
        }
    }

    private void saveTheme(Theme selectedTheme) {

        try {

            preferences.put(PREFERENCE_KEY, selectedTheme.name());
            preferences.flush();

        } catch (BackingStoreException | SecurityException unavailablePreferences) {
            Logger.getLogger(ThemeManager.class.getName())
                    .warning("Appearance changed for this session; preference could not be saved.");
        }
    }

    private void windowsChanged(ListChangeListener.Change<? extends Window> change) {

        while (change.next()) {
            for (Window window : change.getAddedSubList()) {
                configurePopup(window);
            }
        }
    }

    private void configurePopup(Window window) {

        if (!(window instanceof PopupWindow popup)) {
            return;
        }

        if (!owns(popup.getOwnerWindow())) {
            return;
        }

        register(popup.getScene().getRoot());

        if (popup instanceof PopupControl control) {
            registerPopupControl(control);
        }
    }

    private void registerPopupControl(PopupControl control) {

        registerPopupSkin(control);

        if (popups.put(control, true) != null) {
            return;
        }

        control.skinProperty().addListener(
                (observable, previous, current) -> registerPopupSkin(control));
    }

    private void registerPopupSkin(PopupControl popup) {
        if (popup.getSkin() != null && popup.getSkin().getNode() instanceof Parent content) {
            register(content);
        }
    }

    private boolean owns(Window window) {

        if (window == null || window.getScene() == null) {
            return false;
        }

        return window.getScene().getRoot().getProperties().get(STYLE_OWNER) == this;
    }
}
