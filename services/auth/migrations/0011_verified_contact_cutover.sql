LOCK TABLE auth.accounts, auth.account_email_proposals,
    auth.contact_email_verification_attempts IN ACCESS EXCLUSIVE MODE;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM auth.accounts
        WHERE email IS NOT NULL AND
            (length(btrim(email::text)) = 0 OR email::text <> btrim(email::text))
    ) OR EXISTS (
        SELECT 1 FROM auth.accounts a
        JOIN auth.account_email_proposals p ON p.account_id = a.id
        WHERE a.email_verified_at IS NULL AND a.email IS NOT NULL AND p.email <> a.email
    ) THEN
        RAISE EXCEPTION 'Contact cutover requires review of inconsistent legacy addresses'
            USING ERRCODE = '23514';
    END IF;
END;
$$;

INSERT INTO auth.account_email_proposals(account_id, email)
SELECT a.id, a.email FROM auth.accounts a
WHERE a.email_verified_at IS NULL AND a.email IS NOT NULL
  AND NOT EXISTS (SELECT 1 FROM auth.account_email_proposals p WHERE p.account_id = a.id);

ALTER TABLE auth.accounts ALTER COLUMN email DROP NOT NULL;

UPDATE auth.accounts SET email = NULL
WHERE email_verified_at IS NULL AND email IS NOT NULL;

ALTER TABLE auth.accounts
    ADD CONSTRAINT accounts_verified_contact_pair_chk
    CHECK ((email IS NULL AND email_verified_at IS NULL)
        OR (email IS NOT NULL AND email_verified_at IS NOT NULL));

CREATE VIEW auth.session_account_status AS
SELECT s.id, s.account_id, s.created_at, s.expires_at, s.revoked_at,
    COALESCE(a.status = 'active' AND
        CASE s.authentication_method
            WHEN 'google' THEN i.id IS NOT NULL
            WHEN 'password' THEN a.email IS NOT NULL AND a.email_verified_at IS NOT NULL
                AND p.account_id IS NOT NULL
            WHEN 'legacy' THEN a.email IS NOT NULL AND a.email_verified_at IS NOT NULL
                AND p.account_id IS NOT NULL
            ELSE false
        END, false) AS account_allowed
FROM auth.sessions s
JOIN auth.accounts a ON a.id = s.account_id
LEFT JOIN auth.oauth_identities i
    ON i.id = s.oauth_identity_id AND i.account_id = s.account_id AND i.provider = 'google'
LEFT JOIN auth.password_credentials p ON p.account_id = a.id;

CREATE TYPE auth.session_authentication_context AS (
    method VARCHAR(20),
    oauth_identity_id UUID,
    password_version BIGINT,
    authenticated_at TIMESTAMPTZ
);

CREATE FUNCTION auth.open_authenticated_session(
    p_account_id UUID, p_client_type VARCHAR, p_ttl INTERVAL,
    p_authentication auth.session_authentication_context,
    p_device_name VARCHAR DEFAULT NULL, p_ip_address INET DEFAULT NULL,
    p_user_agent TEXT DEFAULT NULL
) RETURNS UUID LANGUAGE plpgsql AS $$
DECLARE
    v_session_id UUID;
    v_now TIMESTAMPTZ;
    v_active BOOLEAN;
    v_contact_verified BOOLEAN;
    v_seconds DOUBLE PRECISION := extract(epoch FROM p_ttl);
BEGIN
    SELECT status = 'active', email_verified_at IS NOT NULL INTO v_active, v_contact_verified
    FROM auth.accounts WHERE id = p_account_id FOR UPDATE;
    IF NOT FOUND THEN RAISE EXCEPTION 'Account does not exist'; END IF;
    IF p_authentication.method IS NULL OR p_authentication.method NOT IN ('legacy', 'password', 'google') THEN
        RAISE EXCEPTION 'Invalid authentication method' USING ERRCODE = '23514';
    END IF;
    IF p_authentication.method = 'legacy' THEN
        IF p_authentication.oauth_identity_id IS NOT NULL OR p_authentication.password_version IS NOT NULL
           OR p_authentication.authenticated_at IS NOT NULL THEN
            RAISE EXCEPTION 'Legacy sessions cannot claim authentication evidence' USING ERRCODE = '23514';
        END IF;
    ELSIF p_authentication.method = 'password' THEN
        IF NOT v_active OR NOT v_contact_verified OR p_authentication.oauth_identity_id IS NOT NULL
           OR p_authentication.password_version IS NULL OR p_authentication.authenticated_at IS NULL
           OR NOT EXISTS (
               SELECT 1 FROM auth.password_credentials WHERE account_id = p_account_id
                 AND (EXTRACT(EPOCH FROM password_changed_at) * 1000000)::bigint = p_authentication.password_version
           ) THEN
            RAISE EXCEPTION 'Password authentication context is not usable' USING ERRCODE = '23514';
        END IF;
    ELSE
        IF NOT v_active OR p_authentication.password_version IS NOT NULL
           OR NOT EXISTS (
               SELECT 1 FROM auth.oauth_identities WHERE id = p_authentication.oauth_identity_id
                 AND account_id = p_account_id AND provider = 'google'
           ) THEN
            RAISE EXCEPTION 'Google authentication context is not usable' USING ERRCODE = '23514';
        END IF;
    END IF;

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
                              user_agent, created_at, last_seen_at, expires_at,
                              authentication_method, oauth_identity_id, password_credential_version,
                              last_authenticated_at, reauthentication_method,
                              reauthentication_password_version, reauthentication_oauth_identity_id)
    VALUES(p_account_id, p_client_type, p_device_name, p_ip_address,
           p_user_agent, v_now, v_now, v_now + make_interval(secs => v_seconds),
           p_authentication.method, p_authentication.oauth_identity_id, p_authentication.password_version,
           p_authentication.authenticated_at,
           CASE WHEN p_authentication.authenticated_at IS NOT NULL THEN p_authentication.method END,
           CASE WHEN p_authentication.authenticated_at IS NOT NULL THEN p_authentication.password_version END,
           CASE WHEN p_authentication.authenticated_at IS NOT NULL THEN p_authentication.oauth_identity_id END)
    RETURNING id INTO v_session_id;
    UPDATE auth.accounts SET last_login_at = v_now WHERE id = p_account_id;
    RETURN v_session_id;
END;
$$;

CREATE OR REPLACE FUNCTION auth.open_session(
    p_account_id UUID, p_client_type VARCHAR, p_ttl INTERVAL,
    p_device_name VARCHAR DEFAULT NULL, p_ip_address INET DEFAULT NULL,
    p_user_agent TEXT DEFAULT NULL
) RETURNS UUID LANGUAGE sql AS $$
    SELECT auth.open_authenticated_session(p_account_id, p_client_type, p_ttl,
        ROW('legacy', NULL, NULL, NULL)::auth.session_authentication_context,
        p_device_name, p_ip_address, p_user_agent);
$$;
