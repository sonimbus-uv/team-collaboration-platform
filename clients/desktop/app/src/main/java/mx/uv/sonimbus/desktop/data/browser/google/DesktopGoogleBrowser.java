package mx.uv.sonimbus.desktop.data.browser.google;

import mx.uv.sonimbus.desktop.data.logging.AuthLog;
import mx.uv.sonimbus.desktop.data.logging.AuthLog.GoogleStage;
import mx.uv.sonimbus.desktop.domain.google.GoogleAttempt;
import mx.uv.sonimbus.desktop.domain.google.GoogleBrowser;
import mx.uv.sonimbus.desktop.domain.google.GoogleError;
import mx.uv.sonimbus.desktop.domain.google.GoogleException;

import java.awt.Desktop;
import java.io.IOException;
import java.time.Duration;
import java.time.Instant;
import java.util.concurrent.CancellationException;
import java.util.concurrent.ExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.TimeoutException;
import java.util.function.BooleanSupplier;

public final class DesktopGoogleBrowser implements GoogleBrowser {
    private volatile GoogleCallbackChannel channel;

    @Override
    public String authorize(GoogleAttempt attempt, BooleanSupplier cancelled)
            throws GoogleException {
        if (cancelled.getAsBoolean()) {
            throw new GoogleException(GoogleError.CANCELLED);
        }
        new WindowsGoogleProtocol().register();
        try (GoogleCallbackChannel active =
                        new GoogleCallbackChannel(
                                attempt.attemptId(), GoogleCallbackChannel.directory());
                GoogleBrowserWindow browserWindow =
                        new GoogleBrowserWindow(attempt.authorizationUrl(), active.result())) {
            channel = active;
            if (cancelled.getAsBoolean()) {
                throw new GoogleException(GoogleError.CANCELLED);
            }
            if (!Desktop.isDesktopSupported()
                    || !Desktop.getDesktop().isSupported(Desktop.Action.BROWSE)) {
                throw new GoogleException(GoogleError.BROWSER);
            }
            Desktop.getDesktop().browse(browserWindow.launchUri());
            AuthLog.googleStage(GoogleStage.BROWSER_OPENED);
            long remaining =
                    Math.min(
                            Duration.ofMinutes(10).toMillis(),
                            Duration.between(Instant.now(), attempt.expiresAt()).toMillis());
            if (remaining <= 0) {
                throw new GoogleException(GoogleError.EXPIRED);
            }
            return active.result().get(remaining, TimeUnit.MILLISECONDS);
        } catch (TimeoutException expired) {
            throw new GoogleException(GoogleError.EXPIRED);
        } catch (CancellationException cancelledFlow) {
            throw new GoogleException(GoogleError.CANCELLED);
        } catch (InterruptedException interrupted) {
            Thread.currentThread().interrupt();
            throw new GoogleException(GoogleError.CANCELLED);
        } catch (ExecutionException failed) {
            if (failed.getCause() instanceof GoogleException known) {
                throw known;
            }
            throw new GoogleException(GoogleError.INVALID_RESPONSE);
        } catch (IOException | UnsupportedOperationException | SecurityException failure) {
            throw new GoogleException(GoogleError.BROWSER, null, failure);
        } finally {
            channel = null;
        }
    }

    @Override
    public void cancel() {
        GoogleCallbackChannel current = channel;
        if (current != null) {
            current.result().cancel(false);
        }
    }
}
