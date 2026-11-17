CREATE TABLE auth.account_security_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES auth.accounts(id),
    event_type VARCHAR(40) NOT NULL DEFAULT 'google_identity_linked'
        CHECK (event_type = 'google_identity_linked'),
    provider VARCHAR(30) NOT NULL DEFAULT 'google' CHECK (provider = 'google'),
    oauth_identity_id UUID NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    read_at TIMESTAMPTZ,
    CONSTRAINT security_event_identity_fk FOREIGN KEY (oauth_identity_id, account_id, provider)
        REFERENCES auth.oauth_identities(id, account_id, provider),
    CONSTRAINT security_event_one_link UNIQUE (event_type, oauth_identity_id),
    CONSTRAINT security_event_result_unique UNIQUE (id, account_id, oauth_identity_id),
    CHECK (read_at IS NULL OR read_at >= created_at)
);

CREATE INDEX security_event_account_time ON auth.account_security_events(account_id, created_at DESC);

ALTER TABLE auth.federated_auth_attempts
    ADD CONSTRAINT federated_result_security_event_fk
    FOREIGN KEY (result_security_event_id, result_account_id, result_identity_id)
    REFERENCES auth.account_security_events(id, account_id, oauth_identity_id);

CREATE TABLE auth.security_notification_outbox (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL UNIQUE REFERENCES auth.account_security_events(id),
    recipient_email CITEXT NOT NULL CHECK (length(btrim(recipient_email::text)) > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    attempt_count INTEGER NOT NULL DEFAULT 0 CHECK (attempt_count >= 0),
    last_attempt_at TIMESTAMPTZ,
    next_attempt_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    delivered_at TIMESTAMPTZ,
    last_error_kind VARCHAR(30) CHECK (last_error_kind IN ('timeout', 'provider_failed', 'unavailable')),
    CHECK ((attempt_count = 0 AND last_attempt_at IS NULL AND delivered_at IS NULL AND last_error_kind IS NULL)
        OR (attempt_count > 0 AND last_attempt_at IS NOT NULL AND last_attempt_at >= created_at)),
    CHECK (next_attempt_at >= created_at),
    CHECK (delivered_at IS NULL OR (delivered_at >= last_attempt_at AND last_error_kind IS NULL))
);

CREATE INDEX security_notification_pending ON auth.security_notification_outbox(next_attempt_at)
    WHERE delivered_at IS NULL;

CREATE FUNCTION auth.protect_account_security_event()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF ROW(NEW.id, NEW.account_id, NEW.event_type, NEW.provider, NEW.oauth_identity_id, NEW.created_at)
       IS DISTINCT FROM ROW(OLD.id, OLD.account_id, OLD.event_type, OLD.provider, OLD.oauth_identity_id, OLD.created_at)
       OR (OLD.read_at IS NOT NULL AND NEW.read_at IS DISTINCT FROM OLD.read_at) THEN
        RAISE EXCEPTION 'Security event is immutable' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_protect_account_security_event
BEFORE UPDATE ON auth.account_security_events
FOR EACH ROW EXECUTE FUNCTION auth.protect_account_security_event();

CREATE FUNCTION auth.protect_security_notification()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NOT EXISTS (
            SELECT 1 FROM auth.account_security_events e
            JOIN auth.accounts a ON a.id = e.account_id
            WHERE e.id = NEW.event_id AND a.email = NEW.recipient_email
              AND a.email_verified_at IS NOT NULL
        ) THEN
            RAISE EXCEPTION 'Notification requires a verified contact' USING ERRCODE = '23514';
        END IF;
    ELSE
        IF ROW(NEW.id, NEW.event_id, NEW.recipient_email, NEW.created_at)
           IS DISTINCT FROM ROW(OLD.id, OLD.event_id, OLD.recipient_email, OLD.created_at)
           OR NEW.attempt_count < OLD.attempt_count
           OR (OLD.last_attempt_at IS NOT NULL AND
               (NEW.last_attempt_at IS NULL OR NEW.last_attempt_at < OLD.last_attempt_at))
           OR (OLD.delivered_at IS NOT NULL AND NEW IS DISTINCT FROM OLD) THEN
            RAISE EXCEPTION 'Notification context is immutable' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_protect_security_notification
BEFORE INSERT OR UPDATE ON auth.security_notification_outbox
FOR EACH ROW EXECUTE FUNCTION auth.protect_security_notification();
