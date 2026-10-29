-- TIMESTAMPTZ + interval '20 days' can mean calendar days across DST.
-- An absolute lifetime must use elapsed seconds regardless of connection time zone.
CREATE OR REPLACE FUNCTION auth.open_session(
    p_account_id UUID, p_client_type VARCHAR, p_ttl INTERVAL,
    p_device_name VARCHAR DEFAULT NULL, p_ip_address INET DEFAULT NULL,
    p_user_agent TEXT DEFAULT NULL
) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE
    v_session_id UUID;
    v_now TIMESTAMPTZ;
    v_seconds DOUBLE PRECISION := extract(epoch FROM p_ttl);
BEGIN
    PERFORM id FROM auth.accounts WHERE id = p_account_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'Account does not exist'; END IF;
    IF v_seconds IS NULL OR v_seconds <= 0 OR v_seconds > 1728000 THEN
        RAISE EXCEPTION 'Session lifetime must be positive and at most 1728000 seconds';
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
           p_user_agent, v_now, v_now, v_now + make_interval(secs => v_seconds))
    RETURNING id INTO v_session_id;
    UPDATE auth.accounts SET last_login_at = v_now WHERE id = p_account_id;
    RETURN v_session_id;
END;
$$;

ALTER TABLE auth.sessions DROP CONSTRAINT sessions_absolute_lifetime_chk;
ALTER TABLE auth.sessions ADD CONSTRAINT sessions_absolute_lifetime_chk
    CHECK(expires_at <= created_at + interval '1728000 seconds') NOT VALID;
