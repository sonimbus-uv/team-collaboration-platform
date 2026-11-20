package mx.uv.sonimbus.desktop.ui.theme;

import static org.junit.jupiter.api.Assertions.*;
import static org.mockito.Mockito.mock;
import static org.mockito.Mockito.when;

import javafx.application.Platform;
import javafx.css.PseudoClass;
import javafx.scene.Node;
import javafx.scene.Parent;
import javafx.scene.Scene;
import javafx.scene.control.Button;
import javafx.scene.control.ContextMenu;
import javafx.scene.control.Hyperlink;
import javafx.scene.control.Label;
import javafx.scene.control.Labeled;
import javafx.scene.control.ListView;
import javafx.scene.control.MenuButton;
import javafx.scene.control.ScrollPane;
import javafx.scene.control.TextInputControl;
import javafx.scene.image.WritableImage;
import javafx.scene.input.KeyCode;
import javafx.scene.layout.Region;
import javafx.scene.paint.Color;
import javafx.scene.text.Text;
import javafx.stage.PopupWindow;
import javafx.stage.Stage;
import javafx.stage.Window;

import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;
import mx.uv.sonimbus.desktop.data.api.auth.AuthApi;
import mx.uv.sonimbus.desktop.data.api.auth.password.HttpPasswordGateway;
import mx.uv.sonimbus.desktop.data.api.auth.registration.HttpRegistrationGateway;
import mx.uv.sonimbus.desktop.data.api.auth.session.HttpAuthenticationGateway;
import mx.uv.sonimbus.desktop.domain.google.GoogleService;
import mx.uv.sonimbus.desktop.domain.password.PasswordService;
import mx.uv.sonimbus.desktop.domain.registration.RegistrationService;
import mx.uv.sonimbus.desktop.domain.session.SessionService;
import mx.uv.sonimbus.desktop.ui.auth.common.AuthMessages;
import mx.uv.sonimbus.desktop.ui.auth.registration.VerifyAccountParams;
import mx.uv.sonimbus.desktop.data.core.Channel;
import mx.uv.sonimbus.desktop.data.core.CoreApi;
import mx.uv.sonimbus.desktop.data.core.Profile;
import mx.uv.sonimbus.desktop.data.realtime.RealtimeClient;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxService;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxStore;
import mx.uv.sonimbus.desktop.ui.common.BackgroundTasks;
import mx.uv.sonimbus.desktop.ui.navigation.ControllerFactory;
import mx.uv.sonimbus.desktop.ui.navigation.View;
import mx.uv.sonimbus.desktop.ui.navigation.ViewManager;

import org.junit.jupiter.api.AfterEach;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.extension.ExtendWith;
import org.testfx.api.FxRobot;
import org.testfx.framework.junit5.ApplicationExtension;
import org.testfx.framework.junit5.Start;

import java.awt.image.BufferedImage;
import java.nio.file.Files;
import java.nio.file.Path;
import java.time.Instant;
import java.util.UUID;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicReference;
import java.util.prefs.Preferences;

import javax.imageio.ImageIO;

@ExtendWith(ApplicationExtension.class)
class ThemeIntegrationTest {

    private Stage stage;
    private ThemeManager themes;
    private ViewManager views;
    private Preferences preferences;
    private final AuthApi authApi = mock(AuthApi.class);

    @Start
    void start(Stage stage) throws Exception {
        this.stage = stage;
        preferences = Preferences.userRoot().node("/sonimbus-theme-test/" + UUID.randomUUID());
        themes = new ThemeManager(preferences);
        ControllerFactory controllers = new ControllerFactory();
        views = new ViewManager(stage, controllers, themes);
        SessionService session = new SessionService(new HttpAuthenticationGateway(authApi));
        CoreApi core = mock(CoreApi.class);
        when(core.myProfile()).thenReturn(java.util.Optional.of(new Profile("test", "Ana", null, null)));
        controllers
                .register(CoreApi.class, core)
                .register(RealtimeClient.class, mock(RealtimeClient.class))
                .register(OutboxService.class, new OutboxService(mock(OutboxStore.class), session))
                .register(GoogleService.class, mock(GoogleService.class))
                .register(
                        PasswordService.class,
                        new PasswordService(new HttpPasswordGateway(authApi), session))
                .register(
                        RegistrationService.class,
                        new RegistrationService(new HttpRegistrationGateway(authApi)))
                .register(SessionService.class, session)
                .register(
                        BackgroundTasks.class,
                        new BackgroundTasks(Runnable::run, Platform::runLater))
                .register(ViewManager.class, views);
        views.show(View.LOGIN);
        stage.setWidth(1200);
        stage.setHeight(780);
        stage.show();
    }

    @AfterEach
    void cleanPreferences() throws Exception {
        preferences.removeNode();
        preferences.flush();
    }

    @Test
    void selectorChangesLiveColorsAndPersistsAcrossNavigationAndReload(FxRobot robot) {
        robot.interact(
                () -> {
                    assertEquals(Theme.LIGHT, themes.getTheme());
                    for (Theme theme : Theme.values()) {
                        views.show(View.LOGIN);
                        stage.getScene().getRoot().applyCss();
                        ThemeSelector selector =
                                (ThemeSelector) stage.getScene().lookup(".theme-selector");
                        selector.setValue(theme);
                        Parent root = stage.getScene().getRoot();
                        root.applyCss();
                        root.layout();
                        assertEquals(theme, themes.getTheme());
                        assertColor(theme == Theme.LIGHT ? "#FAF8FF" : "#191423", background(root));
                        Button login = (Button) root.lookup("#loginButton");
                        assertColor(
                                theme == Theme.LIGHT ? "#5B21B6" : "#C4B5FD", background(login));
                        assertContrast((Color) login.getTextFill(), background(login), 4.5);
                        login.pseudoClassStateChanged(PseudoClass.getPseudoClass("hover"), true);
                        root.applyCss();
                        assertContrast((Color) login.getTextFill(), background(login), 4.5);
                        login.pseudoClassStateChanged(PseudoClass.getPseudoClass("hover"), false);
                        login.setDisable(true);
                        root.applyCss();
                        assertContrast((Color) login.getTextFill(), background(login), 4.5);
                        login.setDisable(false);
                        root.applyCss();
                        snapshot(stage.getScene(), "login-" + theme.name().toLowerCase());

                        login.fire();
                        root.applyCss();
                        root.layout();
                        Label error = (Label) root.lookup("#errorLabel");
                        assertTrue(error.getParent().isVisible());
                        assertColor(
                                theme == Theme.LIGHT ? "#B42318" : "#FFB4AB",
                                (Color) error.getTextFill());
                        assertContrast(
                                (Color) error.getTextFill(),
                                background((Region) root.lookup("#errorBox")),
                                4.5);
                        snapshot(stage.getScene(), "error-" + theme.name().toLowerCase());

                        views.show(View.MAIN);
                        root = stage.getScene().getRoot();
                        root.applyCss();
                        root.layout();
                        assertEquals(
                                theme, ((ThemeSelector) root.lookup(".theme-selector")).getValue());
                        assertColor(
                                theme == Theme.LIGHT ? "#F1EBFA" : "#251C33",
                                background((Region) root.lookup(".channels-column")));
                        @SuppressWarnings("unchecked")
                        ListView<Channel> channels =
                                (ListView<Channel>) root.lookup("#textChannelsList");
                        channels.getItems().setAll(
                                new Channel("c1", "g1", "general", null, "text", "member", null),
                                new Channel("c2", "g1", "equipo", null, "text", "member", null));
                        channels.getSelectionModel().select(0);
                        root.applyCss();
                        root.layout();
                        Region selected = (Region) channels.lookup(".list-cell:selected");
                        assertNotNull(selected);
                        assertColor(
                                theme == Theme.LIGHT ? "#EBDDFF" : "#3C2956", background(selected));
                        assertContrast(
                                (Color)
                                        selected.getBorder()
                                                .getStrokes()
                                                .getFirst()
                                                .getLeftStroke(),
                                background(selected),
                                3);
                        snapshot(stage.getScene(), "main-" + theme.name().toLowerCase());

                        views.showContent(View.PROFILE);
                        root.applyCss();
                        Region confirmation = (Region) root.lookup("#profileSavedBox");
                        Label text = (Label) root.lookup("#profileSavedLabel");
                        assertContrast((Color) text.getTextFill(), background(confirmation), 4.5);
                        assertEquals(theme, new ThemeManager(preferences).getTheme());
                    }
                });
    }

    @Test
    void tabNavigationHasAVisibleFocusIndicator(FxRobot robot) {
        for (Theme theme : Theme.values()) {
            robot.interact(
                    () -> {
                        themes.setTheme(theme);
                        views.show(View.LOGIN);
                    });
            robot.clickOn("#emailField").press(KeyCode.TAB).release(KeyCode.TAB);
            robot.interact(
                    () -> {
                        stage.getScene().getRoot().applyCss();
                        Node focused = stage.getScene().getFocusOwner();
                        assertNotNull(focused);
                        assertNotEquals("emailField", focused.getId());
                        Region region = (Region) focused;
                        assertFalse(region.getBorder().getStrokes().isEmpty());
                        Color focus =
                                (Color)
                                        region.getBorder()
                                                .getStrokes()
                                                .getFirst()
                                                .getBottomStroke();
                        assertColor(theme == Theme.LIGHT ? "#5B21B6" : "#C4B5FD", focus);
                        assertContrast(
                                focus,
                                theme == Theme.LIGHT ? Color.web("#FFFFFF") : Color.web("#231B30"),
                                3);
                    });
        }
    }

    @Test
    void dialogsAndOpenMenusFollowTheCurrentTheme(FxRobot robot) {
        AtomicReference<Throwable> failure = new AtomicReference<>();
        robot.interact(
                () -> {
                    views.show(View.MAIN);
                    for (Theme theme : Theme.values()) {
                        themes.setTheme(theme);
                        Platform.runLater(
                                () -> {
                                    Stage dialog =
                                            (Stage)
                                                    Window.getWindows().stream()
                                                            .filter(
                                                                    window ->
                                                                            window instanceof Stage
                                                                                    && window
                                                                                            != stage)
                                                            .findFirst()
                                                            .orElseThrow();
                                    try {
                                        Parent root = dialog.getScene().getRoot();
                                        root.applyCss();
                                        root.layout();
                                        assertColor(
                                                theme == Theme.LIGHT ? "#FFFFFF" : "#231B30",
                                                background(root));
                                        snapshot(
                                                dialog.getScene(),
                                                "dialog-" + theme.name().toLowerCase());
                                        themes.setTheme(
                                                theme == Theme.LIGHT ? Theme.DARK : Theme.LIGHT);
                                        root.applyCss();
                                        assertColor(
                                                theme == Theme.LIGHT ? "#231B30" : "#FFFFFF",
                                                background(root));
                                    } catch (Throwable error) {
                                        failure.set(error);
                                    } finally {
                                        dialog.close();
                                    }
                                });
                        views.showDialog(View.CREATE_GROUP, "Crear grupo");
                    }
                    themes.setTheme(Theme.DARK);
                    stage.getScene().getRoot().applyCss();
                    stage.getScene().getRoot().layout();
                    MenuButton menu = (MenuButton) stage.getScene().lookup("#addGroupMenuButton");
                    menu.show();
                    PopupWindow popup =
                            (PopupWindow)
                                    Window.getWindows().stream()
                                            .filter(
                                                    window ->
                                                            window instanceof ContextMenu
                                                                    && window.isShowing())
                                            .findFirst()
                                            .orElseThrow();
                    Parent popupRoot = popup.getScene().getRoot();
                    popupRoot.applyCss();
                    popupRoot.layout();
                    Region context = (Region) ((ContextMenu) popup).getSkin().getNode();
                    assertColor("#231B30", background(context));
                    themes.setTheme(Theme.LIGHT);
                    popupRoot.applyCss();
                    assertColor("#FFFFFF", background(context));
                    menu.hide();
                });
        if (failure.get() != null) {
            throw new AssertionError("Dialog theme validation failed", failure.get());
        }
    }

    @Test
    void everyExistingViewLoadsInBothThemes(FxRobot robot) {
        robot.interact(
                () -> {
                    for (Theme theme : Theme.values()) {
                        themes.setTheme(theme);
                        for (View view : View.values()) {
                            views.show(view);
                            Parent root = stage.getScene().getRoot();
                            root.applyCss();
                            root.layout();
                            assertTrue(
                                    root.getStyleClass().contains(theme.styleClass()), view.name());
                            assertReadableLabels(
                                    root,
                                    theme == Theme.LIGHT
                                            ? Color.web("#FAF8FF")
                                            : Color.web("#191423"));
                            if (root.lookup(".theme-selector") instanceof ThemeSelector selector) {
                                assertEquals(theme, selector.getValue(), view.name());
                            }
                        }
                    }
                });
    }

    private static void assertReadableLabels(Node node, Color parentBackground) {
        if (!node.isVisible() || node.isDisabled() || node.getOpacity() < 1) {
            return;
        }
        Color currentBackground = parentBackground;
        if (node instanceof Region region && region.getBackground() != null) {
            for (var fill : region.getBackground().getFills()) {
                if (fill.getFill() instanceof Color color && color.getOpacity() == 1) {
                    currentBackground = color;
                }
            }
        }
        if (node instanceof Labeled label
                && label.getText() != null
                && !label.getText().isBlank()
                && label.getTextFill() instanceof Color foreground) {
            try {
                assertContrast(foreground, currentBackground, 4.5);
            } catch (AssertionError failure) {
                throw new AssertionError(label.getText() + " " + label.getStyleClass(), failure);
            }
        }
        if (node instanceof Parent parent) {
            for (Node child : parent.getChildrenUnmodifiable()) {
                assertReadableLabels(child, currentBackground);
            }
        }
    }

    @Test
    void registrationMessagesWrapWithoutTruncationInSmallWindows(FxRobot robot)
            throws ApiException {
        when(authApi.register("test@example.invalid", "ValidPass123!"))
                .thenThrow(new ApiException(ApiErrorKind.NETWORK, "register", null));

        for (Theme theme : Theme.values()) {
            for (int width : new int[] {480, 360}) {
                robot.interact(
                        () -> {
                            themes.setTheme(theme);
                            views.show(View.REGISTER);
                            stage.setWidth(width);
                            stage.setHeight(600);
                            Scene scene = stage.getScene();
                            scene.getRoot().applyCss();
                            scene.getRoot().layout();
                            ((TextInputControl) scene.lookup("#emailField"))
                                    .setText("test@example.invalid");
                            ((TextInputControl) scene.lookup("#passwordField"))
                                    .setText("ValidPass123!");
                            ((TextInputControl) scene.lookup("#confirmPasswordField"))
                                    .setText("ValidPass123!");
                            ((Button) scene.lookup("#registerButton")).fire();
                        });
                org.testfx.util.WaitForAsyncUtils.waitForFxEvents();
                robot.interact(
                        () -> {
                            Parent root = stage.getScene().getRoot();
                            root.applyCss();
                            root.layout();
                            Label error = (Label) root.lookup("#errorLabel");
                            Label hint = (Label) root.lookup("#passwordHintLabel");
                            assertTrue(root.lookup("#errorBox").isVisible());
                            assertEquals(AuthMessages.PASSWORD_RULE, hint.getText());
                            assertLabelIsComplete(error);
                            assertLabelIsComplete(hint);
                            ScrollPane scroll = (ScrollPane) root;
                            assertTrue(
                                    scroll.getContent().getLayoutBounds().getHeight()
                                            > scroll.getViewportBounds().getHeight());
                            assertTrue(
                                    scroll.getContent().getLayoutBounds().getWidth()
                                            <= scroll.getViewportBounds().getWidth() + 1);
                            snapshot(
                                    stage.getScene(),
                                    "register-error-" + theme.name().toLowerCase() + "-" + width);
                        });
            }
        }
    }

    private static void assertLabelIsComplete(Label label) {
        assertTrue(label.isWrapText());
        assertTrue(label.getHeight() >= label.prefHeight(label.getWidth()) - .5);
        Text renderedText = (Text) label.lookup(".text");
        assertEquals(label.getText(), renderedText.getText());
        assertTrue(label.getHeight() > label.getFont().getSize() * 1.5);
    }

    @Test
    void verificationErrorAndFooterLinksStayCompleteAndCentered(FxRobot robot) throws ApiException {
        UUID attempt = UUID.randomUUID();
        when(authApi.verifyEmail(attempt, "12345678"))
                .thenThrow(new ApiException(ApiErrorKind.VALIDATION, "verify-email", 400, null));

        for (Theme theme : Theme.values()) {
            for (int width : new int[] {480, 360}) {
                robot.interact(
                        () -> {
                            themes.setTheme(theme);
                            views.show(
                                    View.VERIFY_ACCOUNT,
                                    new VerifyAccountParams(
                                            "test@example.invalid",
                                            attempt,
                                            Instant.now().plusSeconds(600),
                                            true));
                            stage.setWidth(width);
                            stage.setHeight(600);
                            Parent root = stage.getScene().getRoot();
                            root.applyCss();
                            root.layout();
                            ((TextInputControl) root.lookup("#codeField")).setText("12345678");
                            ((Button) root.lookup("#verifyButton")).fire();
                        });
                org.testfx.util.WaitForAsyncUtils.waitForFxEvents();
                robot.interact(
                        () -> {
                            Parent root = stage.getScene().getRoot();
                            root.applyCss();
                            root.layout();
                            Label error = (Label) root.lookup("#errorLabel");
                            assertEquals(AuthMessages.INVALID_CODE, error.getText());
                            assertLabelIsComplete(error);
                            Region card = (Region) root.lookup(".auth-card");
                            double center = card.localToScene(card.getLayoutBounds()).getCenterX();
                            for (String id :
                                    new String[] {"#restartRegistrationLink", "#backToLoginLink"}) {
                                Hyperlink link = (Hyperlink) root.lookup(id);
                                assertEquals(
                                        link.getText(), ((Text) link.lookup(".text")).getText());
                                assertEquals(
                                        center,
                                        link.localToScene(link.getLayoutBounds()).getCenterX(),
                                        1);
                            }
                            ScrollPane scroll = (ScrollPane) root;
                            scroll.setVvalue(1);
                            root.layout();
                            snapshot(
                                    stage.getScene(),
                                    "verification-footer-"
                                            + theme.name().toLowerCase()
                                            + "-"
                                            + width);
                        });
            }
        }
    }

    @Test
    void existingMessageBoxesDoNotTruncateLongText(FxRobot robot) {
        String message =
                "No fue posible completar la operación. Revisa los datos y la conexión con el"
                        + " servidor antes de volver a intentarlo. Los cambios todavía no se"
                        + " guardaron.";
        robot.interact(
                () -> {
                    stage.setWidth(480);
                    stage.setHeight(600);
                    for (Theme theme : Theme.values()) {
                        themes.setTheme(theme);
                        for (View view : View.values()) {
                            views.show(view);
                            Parent root = stage.getScene().getRoot();
                            root.applyCss();
                            root.layout();
                            for (Node node :
                                    root.lookupAll(
                                            ".alert-text, .info-text, .success-text,"
                                                    + " .field-error-text")) {
                                if (!(node instanceof Label label)) {
                                    continue;
                                }
                                Parent container = label.getParent();
                                container.setVisible(true);
                                container.setManaged(true);
                                for (Node ancestor = container.getParent();
                                        ancestor != null;
                                        ancestor = ancestor.getParent()) {
                                    if (ancestor.getId() != null && !ancestor.isVisible()) {
                                        ancestor.setVisible(true);
                                        ancestor.setManaged(true);
                                    }
                                }
                                label.setText(message);
                                root.applyCss();
                                root.layout();
                                try {
                                    assertLabelIsComplete(label);
                                    var bounds = label.localToScene(label.getLayoutBounds());
                                    assertTrue(bounds.getMinX() >= -1);
                                    assertTrue(bounds.getMaxX() <= stage.getScene().getWidth() + 1);
                                } catch (AssertionError failure) {
                                    throw new AssertionError(
                                            view.name() + " / " + label.getId(), failure);
                                }
                                container.setVisible(false);
                                container.setManaged(false);
                            }
                        }
                    }
                });
    }

    @Test
    void savedAppearanceIsReadableFromAnotherJvm(FxRobot robot) throws Exception {
        robot.interact(() -> themes.setTheme(Theme.DARK));
        Process process =
                new ProcessBuilder(
                                Path.of(System.getProperty("java.home"), "bin", "java").toString(),
                                "-cp",
                                System.getProperty("java.class.path"),
                                ThemePreferenceProbe.class.getName(),
                                preferences.absolutePath())
                        .redirectErrorStream(true)
                        .start();
        try {
            assertTrue(process.waitFor(10, TimeUnit.SECONDS));
            assertEquals(0, process.exitValue());
            assertEquals("DARK", new String(process.getInputStream().readAllBytes()).strip());
        } finally {
            process.destroyForcibly();
        }
        robot.interact(() -> assertEquals(Theme.DARK, new ThemeManager(preferences).getTheme()));
    }

    private static Color background(javafx.scene.Node node) {
        return (Color) ((Region) node).getBackground().getFills().getFirst().getFill();
    }

    private static void assertColor(String expected, Color actual) {
        assertEquals(Color.web(expected), actual);
    }

    private static void assertContrast(Color foreground, Color background, double minimum) {
        double first = luminance(foreground);
        double second = luminance(background);
        double ratio = (Math.max(first, second) + .05) / (Math.min(first, second) + .05);
        assertTrue(ratio >= minimum, () -> "Contrast " + ratio + " below " + minimum);
    }

    private static double luminance(Color color) {
        return .2126 * linear(color.getRed())
                + .7152 * linear(color.getGreen())
                + .0722 * linear(color.getBlue());
    }

    private static double linear(double channel) {
        return channel <= .04045 ? channel / 12.92 : Math.pow((channel + .055) / 1.055, 2.4);
    }

    private static void snapshot(Scene scene, String name) {
        try {
            Path directory = Path.of("target", "theme-previews");
            Files.createDirectories(directory);
            WritableImage image = scene.snapshot(null);
            BufferedImage output =
                    new BufferedImage(
                            (int) image.getWidth(),
                            (int) image.getHeight(),
                            BufferedImage.TYPE_INT_ARGB);
            for (int y = 0; y < image.getHeight(); y++) {
                for (int x = 0; x < image.getWidth(); x++) {
                    output.setRGB(x, y, image.getPixelReader().getArgb(x, y));
                }
            }
            ImageIO.write(output, "png", directory.resolve(name + ".png").toFile());
        } catch (Exception error) {
            throw new AssertionError("Cannot capture theme preview", error);
        }
    }
}
