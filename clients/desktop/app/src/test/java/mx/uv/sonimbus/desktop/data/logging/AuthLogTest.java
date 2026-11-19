package mx.uv.sonimbus.desktop.data.logging;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

import mx.uv.sonimbus.desktop.data.api.ApiErrorKind;
import mx.uv.sonimbus.desktop.data.api.ApiException;

import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.io.TempDir;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.logging.FileHandler;
import java.util.logging.Level;
import java.util.logging.LogRecord;
import java.util.logging.Logger;

class AuthLogTest {

    @TempDir Path logDirectory;

    @Test
    void fileOutputExcludesExceptionMessagesAndStackTraces() throws IOException {
        LogRecord record =
                new LogRecord(
                        Level.WARNING,
                        "event=request-failed operation=register error_kind=NETWORK");
        record.setThrown(new IOException("password=Private123! code=00123456"));

        FileHandler handler = AuthLog.createFileHandler(logDirectory);
        try {
            handler.publish(record);
            handler.flush();
        } finally {
            handler.close();
        }

        String log = Files.readString(logDirectory.resolve("auth-0.log"));
        assertTrue(log.contains("event=request-failed operation=register error_kind=NETWORK"));
        assertFalse(log.contains("Private123!"));
        assertFalse(log.contains("00123456"));
        assertFalse(log.contains("IOException"));
        assertFalse(log.contains("\tat "));
    }

    @Test
    void transportFailuresExposeOnlyTheCategoryAndExceptionType() throws IOException {
        FileHandler handler = AuthLog.createFileHandler(logDirectory);
        Logger logger = Logger.getLogger(AuthLog.class.getName());
        logger.addHandler(handler);
        try {
            AuthLog.requestFailed(
                    "register",
                    new ApiException(
                            ApiErrorKind.NETWORK,
                            "register",
                            new IOException("private-user@example.invalid password=Private123!")));
        } finally {
            logger.removeHandler(handler);
            handler.close();
        }

        String log = Files.readString(logDirectory.resolve("auth-0.log"));
        assertTrue(log.contains("error_kind=NETWORK http_status=0 error_type=IOException"));
        assertFalse(log.contains("private-user@example.invalid"));
        assertFalse(log.contains("Private123!"));
        assertFalse(log.contains("\tat "));
    }

    @Test
    void fileOutputAppendsExistingEvents() throws IOException {
        FileHandler firstHandler = AuthLog.createFileHandler(logDirectory);
        try {
            firstHandler.publish(
                    new LogRecord(Level.INFO, "event=request-started operation=register"));
        } finally {
            firstHandler.close();
        }
        FileHandler secondHandler = AuthLog.createFileHandler(logDirectory);
        try {
            secondHandler.publish(
                    new LogRecord(Level.INFO, "event=request-succeeded operation=register"));
        } finally {
            secondHandler.close();
        }

        String log = Files.readString(logDirectory.resolve("auth-0.log"));
        assertTrue(log.contains("event=request-started"));
        assertTrue(log.contains("event=request-succeeded"));
    }
}
