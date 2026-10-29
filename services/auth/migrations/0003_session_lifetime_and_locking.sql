-- Do not rewrite 0001. Use wall-clock time after waiting for locks.
CREATE OR REPLACE FUNCTION auth.open_session(
    p_account_id UUID, p_client_type VARCHAR, p_ttl INTERVAL,
    p_device_name VARCHAR DEFAULT NULL, p_ip_address INET DEFAULT NULL,
    p_user_agent TEXT DEFAULT NULL
) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE
    v_session_id UUID;
    v_now TIMESTAMPTZ;
BEGIN
    -- All session operations lock account before session/token records.
    PERFORM id FROM auth.accounts WHERE id = p_account_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'Account does not exist'; END IF;
    IF p_ttl <= interval '0 seconds' OR p_ttl > interval '20 days' OR p_ttl IS NULL THEN
        RAISE EXCEPTION 'Session lifetime must be positive and at most 20 days';
    END IF;
    PERFORM pg_advisory_xact_lock(hashtextextended(p_account_id::text || ':' || p_client_type, 0));
    v_now := date_trunc('second', clock_timestamp());
    UPDATE auth.sessions
       SET revoked_at = v_now,
           revoke_reason = CASE WHEN expires_at <= v_now THEN 'expired' ELSE 'superseded' END
     WHERE account_id = p_account_id AND client_type = p_client_type AND revoked_at IS NULL;
    INSERT INTO auth.sessions(account_id, client_type, device_name, ip_address,
                              user_agent, created_at, last_seen_at, expires_at)
    VALUES(p_account_id, p_client_type, p_device_name, p_ip_address,
           p_user_agent, v_now, v_now, v_now + p_ttl)
    RETURNING id INTO v_session_id;
    UPDATE auth.accounts SET last_login_at = v_now WHERE id = p_account_id;
    RETURN v_session_id;
END;
$$;

-- Validate new writes without rewriting preexisting data.
ALTER TABLE auth.sessions ADD CONSTRAINT sessions_absolute_lifetime_chk
    CHECK(expires_at <= created_at + interval '20 days') NOT VALID;

CREATE FUNCTION auth.protect_session_identity()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF ROW(NEW.id, NEW.account_id, NEW.client_type, NEW.created_at, NEW.expires_at)
       IS DISTINCT FROM ROW(OLD.id, OLD.account_id, OLD.client_type, OLD.created_at, OLD.expires_at)
       OR (OLD.revoked_at IS NOT NULL AND
           ROW(NEW.revoked_at, NEW.revoke_reason) IS DISTINCT FROM ROW(OLD.revoked_at, OLD.revoke_reason)) THEN
        RAISE EXCEPTION 'Session identity, absolute expiration and revocation are immutable';
    END IF;
    RETURN NEW;
END;
$$;
CREATE TRIGGER trg_protect_session_identity
BEFORE UPDATE ON auth.sessions
FOR EACH ROW EXECUTE FUNCTION auth.protect_session_identity();
