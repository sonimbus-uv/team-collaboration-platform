ALTER TABLE auth.oauth_identities
    ADD CONSTRAINT oauth_identity_account_provider_unique UNIQUE (id, account_id, provider);

ALTER TABLE auth.sessions
    ADD COLUMN authentication_method VARCHAR(20) NOT NULL DEFAULT 'legacy',
    ADD COLUMN oauth_identity_id UUID,
    ADD COLUMN password_credential_version BIGINT,
    ADD COLUMN last_authenticated_at TIMESTAMPTZ,
    ADD COLUMN reauthentication_method VARCHAR(20),
    ADD COLUMN reauthentication_password_version BIGINT,
    ADD COLUMN reauthentication_oauth_identity_id UUID,
    ADD CONSTRAINT sessions_id_account_unique UNIQUE (id, account_id),
    ADD CONSTRAINT sessions_authentication_method_chk CHECK (
        (authentication_method = 'legacy' AND oauth_identity_id IS NULL AND password_credential_version IS NULL)
        OR (authentication_method = 'password' AND oauth_identity_id IS NULL
            AND password_credential_version IS NOT NULL AND password_credential_version > 0)
        OR (authentication_method = 'google' AND oauth_identity_id IS NOT NULL AND password_credential_version IS NULL)
    ),
    ADD CONSTRAINT sessions_authentication_identity_fk
        FOREIGN KEY (oauth_identity_id, account_id, authentication_method)
        REFERENCES auth.oauth_identities(id, account_id, provider),
    ADD CONSTRAINT sessions_reauthentication_chk CHECK (
        (last_authenticated_at IS NULL AND reauthentication_method IS NULL
            AND reauthentication_password_version IS NULL AND reauthentication_oauth_identity_id IS NULL)
        OR (last_authenticated_at IS NOT NULL AND reauthentication_method IS NOT NULL
            AND reauthentication_method = 'password' AND reauthentication_password_version IS NOT NULL
            AND reauthentication_password_version > 0 AND reauthentication_oauth_identity_id IS NULL)
        OR (last_authenticated_at IS NOT NULL AND reauthentication_method IS NOT NULL
            AND reauthentication_method = 'google'
            AND reauthentication_password_version IS NULL AND reauthentication_oauth_identity_id IS NOT NULL)
    ),
    ADD CONSTRAINT sessions_reauthentication_identity_fk
        FOREIGN KEY (reauthentication_oauth_identity_id, account_id, reauthentication_method)
        REFERENCES auth.oauth_identities(id, account_id, provider);

CREATE INDEX sessions_authentication_identity_idx ON auth.sessions(oauth_identity_id)
    WHERE oauth_identity_id IS NOT NULL;
CREATE INDEX sessions_reauthentication_identity_idx ON auth.sessions(reauthentication_oauth_identity_id)
    WHERE reauthentication_oauth_identity_id IS NOT NULL;

CREATE OR REPLACE FUNCTION auth.protect_session_identity()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF ROW(NEW.id, NEW.account_id, NEW.client_type, NEW.created_at, NEW.expires_at,
           NEW.authentication_method, NEW.oauth_identity_id, NEW.password_credential_version)
       IS DISTINCT FROM ROW(OLD.id, OLD.account_id, OLD.client_type, OLD.created_at, OLD.expires_at,
           OLD.authentication_method, OLD.oauth_identity_id, OLD.password_credential_version)
       OR (OLD.revoked_at IS NOT NULL AND
           ROW(NEW.revoked_at, NEW.revoke_reason) IS DISTINCT FROM ROW(OLD.revoked_at, OLD.revoke_reason)) THEN
        RAISE EXCEPTION 'Session identity, authentication and revocation are immutable';
    END IF;
    RETURN NEW;
END;
$$;

CREATE FUNCTION auth.protect_session_reauthentication()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.last_authenticated_at > clock_timestamp()
           OR (NEW.last_authenticated_at IS NOT NULL AND
               (NEW.revoked_at IS NOT NULL OR NEW.expires_at <= clock_timestamp())) THEN
            RAISE EXCEPTION 'Invalid initial authentication evidence' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF ROW(NEW.last_authenticated_at, NEW.reauthentication_method,
           NEW.reauthentication_password_version, NEW.reauthentication_oauth_identity_id)
       IS DISTINCT FROM ROW(OLD.last_authenticated_at, OLD.reauthentication_method,
           OLD.reauthentication_password_version, OLD.reauthentication_oauth_identity_id) THEN
        IF OLD.revoked_at IS NOT NULL OR NEW.revoked_at IS NOT NULL
           OR OLD.expires_at <= clock_timestamp()
           OR NEW.last_authenticated_at IS NULL
           OR NEW.last_authenticated_at > clock_timestamp()
           OR (OLD.last_authenticated_at IS NOT NULL AND NEW.last_authenticated_at < OLD.last_authenticated_at) THEN
            RAISE EXCEPTION 'Invalid reauthentication evidence' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_protect_session_reauthentication
BEFORE INSERT OR UPDATE ON auth.sessions
FOR EACH ROW EXECUTE FUNCTION auth.protect_session_reauthentication();
