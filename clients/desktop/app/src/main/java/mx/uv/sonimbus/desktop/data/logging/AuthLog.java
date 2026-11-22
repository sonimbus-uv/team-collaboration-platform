package mx.uv.sonimbus.desktop.data.logging;

import mx.uv.sonimbus.desktop.data.api.ApiException;

import java.io.IOException;
import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.logging.ConsoleHandler;
import java.util.logging.ErrorManager;
import java.util.logging.FileHandler;
import java.util.logging.Formatter;
import java.util.logging.Handler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;

public final class AuthLog {

    private static final int MAX_FILE_BYTES = 1_048_576;
    private static final int FILE_COUNT = 3;
    private static final Logger LOGGER = createLogger();

    private AuthLog() {}

    public static void requestStarted(String operation) {
        LOGGER.info(() -> "event=request-started operation=" + operation);
    }

    public static void requestSucceeded(String operation, int statusCode) {
        LOGGER.info(
                () ->
                        "event=request-succeeded operation="
                                + operation
                                + " http_status="
                                + statusCode);
    }

    public static void requestFailed(String operation, ApiException failure) {
        LOGGER.warning(
                () ->
                        "event=request-failed operation="
                                + operation
                                + " error_kind="
                                + failure.kind()
                                + " http_status="
                                + failure.status()
                                + " error_type="
                                + errorType(failure.getCause()));
    }

    public static void unexpectedFailure(String operation, Throwable failure) {
        LOGGER.warning(
                () ->
                        "event=unexpected-failure operation="
                                + operation
                                + " error_type="
                                + errorType(failure));
    }

    public static void invalidResponse(String operation) {
        LOGGER.warning(() -> "event=invalid-response operation=" + operation);
    }

    public enum GoogleStage {
        BROWSER_OPENED,
        CALLBACK_RECEIVED,
        CALLBACK_CANCELLED,
        CALLBACK_REJECTED,
        RETURN_DELIVERED,
        RETURN_UNAVAILABLE
    }

    public static void googleStage(GoogleStage stage) {
        LOGGER.info(() -> "event=google-stage stage=" + stage);
    }

    private static String errorType(Throwable failure) {
        return failure == null ? "none" : failure.getClass().getSimpleName();
    }

    private static Logger createLogger() {

        Logger logger = Logger.getLogger(AuthLog.class.getName());
        logger.setUseParentHandlers(false);
        logger.setLevel(Level.INFO);

        try {

            Path directory = Path.of(System.getProperty("user.home"), ".sonimbus", "logs");
            logger.addHandler(createFileHandler(directory));

        } catch (IOException | SecurityException initializationFailure) {
            Handler console = new ConsoleHandler();
            console.setFormatter(new EventFormatter());
            console.setErrorManager(new SafeErrorManager());
            logger.addHandler(console);
            logger.warning(
                    () ->
                            "event=log-file-unavailable error_type="
                                    + errorType(initializationFailure));
        }

        return logger;
    }

    static FileHandler createFileHandler(Path directory) throws IOException {
        Files.createDirectories(directory);
        FileHandler handler =
                new FileHandler(
                        directory.resolve("auth-%g.log").toString(),
                        MAX_FILE_BYTES,
                        FILE_COUNT,
                        true);
        handler.setEncoding(StandardCharsets.UTF_8.name());
        handler.setFormatter(new EventFormatter());
        handler.setErrorManager(new SafeErrorManager());

        return handler;
    }

    private static final class SafeErrorManager extends ErrorManager {
        @Override
        public void error(String message, Exception failure, int errorCode) {
            System.err.println("Auth logging unavailable; error_code=" + errorCode);
        }
    }

    private static final class EventFormatter extends Formatter {
        @Override
        public String format(LogRecord logRecord) {
            return logRecord.getInstant()
                    + " "
                    + logRecord.getLevel()
                    + " "
                    + logRecord.getMessage()
                    + "\n";
        }
    }
}
