package mx.uv.sonimbus.desktop.data.browser.google;

import mx.uv.sonimbus.desktop.domain.google.GoogleError;
import mx.uv.sonimbus.desktop.domain.google.GoogleException;

import java.io.File;
import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.Arrays;
import java.util.Locale;
import java.util.concurrent.TimeUnit;
import java.util.stream.Collectors;

public final class WindowsGoogleProtocol {
    public void register() throws GoogleException {
        if (!System.getProperty("os.name").toLowerCase(Locale.ROOT).startsWith("windows")) {
            throw new GoogleException(GoogleError.PLATFORM);
        }
        try {
            importRegistration();
        } catch (IOException | InterruptedException failure) {
            if (failure instanceof InterruptedException) {
                Thread.currentThread().interrupt();
            }
            throw new GoogleException(GoogleError.PLATFORM, null, failure);
        }
    }

    private void importRegistration() throws IOException, InterruptedException {
        Path registration = Files.createTempFile("sonimbus-protocol-", ".reg");
        try {
            Files.writeString(
                    registration, registrationText(launchCommand()), StandardCharsets.UTF_16LE);
            Process process =
                    new ProcessBuilder("reg.exe", "import", registration.toString())
                            .redirectOutput(ProcessBuilder.Redirect.DISCARD)
                            .redirectError(ProcessBuilder.Redirect.DISCARD)
                            .start();
            if (!process.waitFor(10, TimeUnit.SECONDS)) {
                process.destroyForcibly();
                throw new IOException("Protocol registration timed out");
            }
            if (process.exitValue() != 0) {
                throw new IOException("Protocol registration failed");
            }
        } finally {
            Files.deleteIfExists(registration);
        }
    }

    static String registrationText(String command) {
        String escaped = command.replace("\\", "\\\\").replace("\"", "\\\"");
        return "\ufeffWindows Registry Editor Version 5.00\r\n\r\n"
                + "[HKEY_CURRENT_USER\\Software\\Classes\\sonimbus]\r\n"
                + "@=\"URL:Sonimbus\"\r\n\"URL Protocol\"=\"\"\r\n\r\n"
                + "[HKEY_CURRENT_USER\\Software\\Classes\\sonimbus\\shell\\open\\command]\r\n"
                + "@=\""
                + escaped
                + "\"\r\n";
    }

    static String launchCommand() {
        Path java = Path.of(System.getProperty("java.home"), "bin", "javaw.exe").toAbsolutePath();
        String classpath =
                Arrays.stream(System.getProperty("java.class.path").split(File.pathSeparator))
                        .map(entry -> Path.of(entry).toAbsolutePath().normalize().toString())
                        .collect(Collectors.joining(File.pathSeparator));
        return quote(java.toString())
                + " -cp "
                + quote(classpath)
                + " "
                + GoogleCallbackLauncher.class.getName()
                + " \"%1\"";
    }

    private static String quote(String value) {
        if (value.contains("\"") || value.contains("\n") || value.contains("\r")) {
            throw new IllegalArgumentException("Invalid protocol executable path");
        }
        return "\"" + value + "\"";
    }
}
