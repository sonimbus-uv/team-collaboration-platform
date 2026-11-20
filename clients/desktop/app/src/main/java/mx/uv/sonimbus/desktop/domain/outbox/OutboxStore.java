package mx.uv.sonimbus.desktop.domain.outbox;

import java.util.List;

public interface OutboxStore {

    void add(String accountId, OutboxMessage message);

    List<OutboxMessage> unsent(String accountId, String channelId);

    void markFailed(String accountId, String clientMessageId);

    void remove(String accountId, String clientMessageId);
}
