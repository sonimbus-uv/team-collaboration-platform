-- Minimal transactional intent only. No broker worker or delivery claim yet.
CREATE TABLE auth.session_revocation_outbox (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    session_id UUID NOT NULL UNIQUE,
    occurred_at TIMESTAMPTZ NOT NULL,
    event JSONB NOT NULL
);

CREATE FUNCTION auth.record_session_revocation()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    v_event_id UUID := gen_random_uuid();
BEGIN
    IF OLD.revoked_at IS NULL AND NEW.revoked_at IS NOT NULL THEN
        UPDATE auth.refresh_tokens
           SET revoked_at = COALESCE(revoked_at, NEW.revoked_at)
         WHERE session_id = NEW.id;
        INSERT INTO auth.session_revocation_outbox(event_id, session_id, occurred_at, event)
        VALUES (v_event_id, NEW.id, NEW.revoked_at,
            jsonb_build_object(
                'type', 'session.revoked', 'version', 1,
                'event_id', v_event_id, 'occurred_at', NEW.revoked_at,
                'payload', jsonb_build_object(
                    'sid', NEW.id, 'account_id', NEW.account_id,
                    'client_type', NEW.client_type, 'reason', NEW.revoke_reason,
                    'revoked_at', NEW.revoked_at, 'session_expires_at', NEW.expires_at)));
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_session_revocation_outbox
AFTER UPDATE OF revoked_at ON auth.sessions
FOR EACH ROW EXECUTE FUNCTION auth.record_session_revocation();
