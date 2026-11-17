CREATE INDEX security_notification_ready
    ON auth.security_notification_outbox(next_attempt_at, id)
    WHERE delivered_at IS NULL AND attempt_count < 10;
