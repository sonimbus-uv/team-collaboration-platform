package mx.uv.sonimbus.desktop.data.local;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.sql.Connection;
import java.sql.DriverManager;
import java.sql.PreparedStatement;
import java.sql.ResultSet;
import java.sql.SQLException;
import java.sql.Statement;
import java.time.Instant;
import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxException;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxMessage;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxStatus;
import mx.uv.sonimbus.desktop.domain.outbox.OutboxStore;

public final class SqliteOutboxStore implements OutboxStore {

    private static final int BUSY_TIMEOUT_MILLIS = 5000;

    private final String url;

    public SqliteOutboxStore(Path file) {
        Objects.requireNonNull(file);
        try {
            Files.createDirectories(file.toAbsolutePath().getParent());
            if (Files.notExists(file)) {
                PrivateLocalFile.create(file.toAbsolutePath());
            }
        } catch (IOException e) {
            throw new OutboxException("Local outbox unavailable", e);
        }
        this.url = "jdbc:sqlite:" + file.toAbsolutePath();
        migrate();
    }

    @Override
    public void add(String accountId, OutboxMessage message) {
        execute("INSERT INTO outbox (account_id, client_message_id, channel_id, content, status, created_at)"
                + " VALUES (?, ?, ?, ?, ?, ?)", statement -> {
                    statement.setString(1, accountId);
                    statement.setString(2, message.clientMessageId());
                    statement.setString(3, message.channelId());
                    statement.setString(4, message.content());
                    statement.setString(5, message.status().name());
                    statement.setString(6, message.createdAt().toString());
                });
    }

    @Override
    public List<OutboxMessage> unsent(String accountId, String channelId) {
        try (Connection connection = open();
                PreparedStatement statement = connection.prepareStatement(
                        "SELECT client_message_id, channel_id, content, status, created_at FROM outbox"
                                + " WHERE account_id = ? AND channel_id = ? ORDER BY seq")) {
            statement.setString(1, accountId);
            statement.setString(2, channelId);
            List<OutboxMessage> result = new ArrayList<>();
            try (ResultSet rows = statement.executeQuery()) {
                while (rows.next()) {
                    result.add(new OutboxMessage(
                            rows.getString(1),
                            rows.getString(2),
                            rows.getString(3),
                            OutboxStatus.valueOf(rows.getString(4)),
                            Instant.parse(rows.getString(5))));
                }
            }
            return result;
        } catch (SQLException e) {
            throw new OutboxException("Local outbox read failed", e);
        }
    }

    @Override
    public void markFailed(String accountId, String clientMessageId) {
        execute("UPDATE outbox SET status = ? WHERE account_id = ? AND client_message_id = ?", statement -> {
            statement.setString(1, OutboxStatus.FAILED.name());
            statement.setString(2, accountId);
            statement.setString(3, clientMessageId);
        });
    }

    @Override
    public void remove(String accountId, String clientMessageId) {
        execute("DELETE FROM outbox WHERE account_id = ? AND client_message_id = ?", statement -> {
            statement.setString(1, accountId);
            statement.setString(2, clientMessageId);
        });
    }

    private void migrate() {
        try (Connection connection = open(); Statement statement = connection.createStatement()) {
            statement.execute("CREATE TABLE IF NOT EXISTS outbox ("
                    + "seq INTEGER PRIMARY KEY AUTOINCREMENT, "
                    + "account_id TEXT NOT NULL, "
                    + "client_message_id TEXT NOT NULL, "
                    + "channel_id TEXT NOT NULL, "
                    + "content TEXT NOT NULL, "
                    + "status TEXT NOT NULL, "
                    + "created_at TEXT NOT NULL, "
                    + "UNIQUE (account_id, client_message_id))");
        } catch (SQLException e) {
            throw new OutboxException("Local outbox migration failed", e);
        }
    }

    private Connection open() throws SQLException {
        Connection connection = DriverManager.getConnection(url);
        try (Statement statement = connection.createStatement()) {
            statement.execute("PRAGMA busy_timeout = " + BUSY_TIMEOUT_MILLIS);
        } catch (SQLException e) {
            connection.close();
            throw e;
        }
        return connection;
    }

    private void execute(String sql, Binder binder) {
        try (Connection connection = open(); PreparedStatement statement = connection.prepareStatement(sql)) {
            binder.bind(statement);
            statement.executeUpdate();
        } catch (SQLException e) {
            throw new OutboxException("Local outbox write failed", e);
        }
    }

    @FunctionalInterface
    private interface Binder {
        void bind(PreparedStatement statement) throws SQLException;
    }
}
