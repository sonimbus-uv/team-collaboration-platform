CREATE TABLE auth.federated_auth_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    purpose VARCHAR(10) NOT NULL CHECK (purpose IN ('login', 'link')),
    provider VARCHAR(30) NOT NULL DEFAULT 'google' CHECK (provider = 'google'),
    client_type VARCHAR(30) NOT NULL CHECK (client_type IN ('desktop', 'mobile')),
    device_name VARCHAR(120),
    redirect_target VARCHAR(64) NOT NULL CHECK (length(btrim(redirect_target)) > 0),
    initiator_secret_hash BYTEA NOT NULL CHECK (octet_length(initiator_secret_hash) = 32),
    launch_ticket_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(launch_ticket_hash) = 32),
    oauth_state_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(oauth_state_hash) = 32),
    oidc_nonce_hash BYTEA NOT NULL UNIQUE CHECK (octet_length(oidc_nonce_hash) = 32),
    code_challenge BYTEA NOT NULL CHECK (octet_length(code_challenge) = 32),
    initiator_account_id UUID REFERENCES auth.accounts(id),
    initiator_session_id UUID,
    initiator_authenticated_at TIMESTAMPTZ,
    initiator_authentication_method VARCHAR(20),
    initiator_credential_version BIGINT,
    status VARCHAR(20) NOT NULL DEFAULT 'created'
        CHECK (status IN ('created', 'authorizing', 'verified', 'consumed', 'cancelled', 'failed', 'expired')),
    created_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp() + interval '10 minutes',
    authorization_started_at TIMESTAMPTZ,
    provider_subject TEXT,
    provider_email CITEXT,
    email_authoritative BOOLEAN NOT NULL DEFAULT false,
    identity_verified_at TIMESTAMPTZ,
    identity_valid_until TIMESTAMPTZ,
    completion_code_hash BYTEA UNIQUE CHECK (completion_code_hash IS NULL OR octet_length(completion_code_hash) = 32),
    completion_issued_at TIMESTAMPTZ,
    completion_expires_at TIMESTAMPTZ,
    terminal_at TIMESTAMPTZ,
    result_account_id UUID REFERENCES auth.accounts(id),
    result_identity_id UUID,
    result_session_id UUID,
    result_security_event_id UUID,
    CONSTRAINT federated_initiator_session_fk FOREIGN KEY (initiator_session_id, initiator_account_id)
        REFERENCES auth.sessions(id, account_id),
    CONSTRAINT federated_result_identity_fk FOREIGN KEY (result_identity_id, result_account_id, provider)
        REFERENCES auth.oauth_identities(id, account_id, provider),
    CONSTRAINT federated_result_session_fk FOREIGN KEY (result_session_id, result_account_id)
        REFERENCES auth.sessions(id, account_id),
    CONSTRAINT federated_lifetime_chk CHECK (
        expires_at > created_at AND expires_at <= created_at + interval '10 minutes'
    ),
    CONSTRAINT federated_initiator_chk CHECK (
        (purpose = 'login' AND initiator_account_id IS NULL AND initiator_session_id IS NULL
            AND initiator_authenticated_at IS NULL AND initiator_authentication_method IS NULL
            AND initiator_credential_version IS NULL)
        OR (purpose = 'link' AND initiator_account_id IS NOT NULL AND initiator_session_id IS NOT NULL
            AND initiator_authenticated_at IS NOT NULL
            AND initiator_authenticated_at BETWEEN created_at - interval '10 minutes' AND created_at
            AND initiator_authentication_method IS NOT NULL
            AND ((initiator_authentication_method = 'password' AND initiator_credential_version IS NOT NULL
                    AND initiator_credential_version > 0)
                OR (initiator_authentication_method = 'google' AND initiator_credential_version IS NULL)))
    ),
    CONSTRAINT federated_authorization_started_chk CHECK (
        (status = 'created' AND authorization_started_at IS NULL)
        OR (status IN ('authorizing', 'verified', 'consumed') AND authorization_started_at IS NOT NULL
            AND authorization_started_at >= created_at AND authorization_started_at < expires_at)
        OR (status IN ('cancelled', 'failed', 'expired') AND
            (authorization_started_at IS NULL OR authorization_started_at >= created_at))
    ),
    CONSTRAINT federated_verified_proof_chk CHECK (
        (provider_subject IS NULL AND provider_email IS NULL AND NOT email_authoritative
            AND identity_verified_at IS NULL AND identity_valid_until IS NULL
            AND completion_code_hash IS NULL AND completion_issued_at IS NULL AND completion_expires_at IS NULL
            AND status NOT IN ('verified', 'consumed'))
        OR (provider_subject IS NOT NULL AND length(provider_subject) BETWEEN 1 AND 255
            AND identity_verified_at IS NOT NULL AND identity_valid_until IS NOT NULL
            AND identity_valid_until > identity_verified_at
            AND authorization_started_at IS NOT NULL AND identity_verified_at >= authorization_started_at
            AND completion_code_hash IS NOT NULL AND completion_issued_at IS NOT NULL
            AND completion_expires_at IS NOT NULL AND completion_issued_at >= identity_verified_at
            AND completion_expires_at > completion_issued_at
            AND completion_expires_at <= completion_issued_at + interval '60 seconds'
            AND completion_expires_at <= expires_at AND completion_expires_at <= identity_valid_until
            AND (NOT email_authoritative OR (provider_email IS NOT NULL AND length(btrim(provider_email::text)) > 0))
            AND status IN ('verified', 'consumed', 'cancelled', 'failed', 'expired'))
    ),
    CONSTRAINT federated_terminal_chk CHECK (
        (status IN ('created', 'authorizing', 'verified') AND terminal_at IS NULL)
        OR (status IN ('consumed', 'cancelled', 'failed', 'expired')
            AND terminal_at IS NOT NULL AND terminal_at >= created_at)
    ),
    CONSTRAINT federated_result_chk CHECK (
        (status <> 'consumed' AND result_account_id IS NULL AND result_identity_id IS NULL
            AND result_session_id IS NULL AND result_security_event_id IS NULL)
        OR (status = 'consumed' AND result_account_id IS NOT NULL AND result_identity_id IS NOT NULL
            AND terminal_at <= completion_expires_at
            AND ((purpose = 'login' AND result_session_id IS NOT NULL AND result_security_event_id IS NULL)
                OR (purpose = 'link' AND result_session_id IS NULL AND result_security_event_id IS NOT NULL
                    AND result_account_id = initiator_account_id)))
    )
);

CREATE INDEX federated_attempt_cleanup ON auth.federated_auth_attempts(expires_at);
CREATE INDEX federated_attempt_initiator ON auth.federated_auth_attempts(initiator_session_id)
    WHERE initiator_session_id IS NOT NULL;

CREATE FUNCTION auth.protect_federated_auth_attempt()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        IF NEW.status <> 'created' OR NEW.created_at > clock_timestamp() THEN
            RAISE EXCEPTION 'Federated attempts must start unverified' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    IF NEW.status IN ('authorizing', 'verified', 'consumed') AND NEW.expires_at <= clock_timestamp() THEN
        RAISE EXCEPTION 'Federated attempt expired' USING ERRCODE = '23514';
    END IF;
    IF NEW.status IN ('verified', 'consumed') AND NEW.completion_expires_at <= clock_timestamp() THEN
        RAISE EXCEPTION 'Completion proof expired' USING ERRCODE = '23514';
    END IF;
    IF NEW IS NOT DISTINCT FROM OLD THEN
        RETURN NEW;
    END IF;
    IF NOT ((OLD.status = 'created' AND NEW.status IN ('authorizing', 'cancelled', 'failed', 'expired'))
         OR (OLD.status = 'authorizing' AND NEW.status IN ('verified', 'cancelled', 'failed', 'expired'))
         OR (OLD.status = 'verified' AND NEW.status IN ('consumed', 'cancelled', 'failed', 'expired'))) THEN
        RAISE EXCEPTION 'Invalid federated attempt transition' USING ERRCODE = '23514';
    END IF;
    IF ROW(NEW.id, NEW.purpose, NEW.provider, NEW.client_type, NEW.device_name, NEW.redirect_target,
           NEW.initiator_secret_hash, NEW.launch_ticket_hash, NEW.oauth_state_hash, NEW.oidc_nonce_hash,
           NEW.code_challenge, NEW.initiator_account_id, NEW.initiator_session_id,
           NEW.initiator_authenticated_at, NEW.initiator_authentication_method, NEW.initiator_credential_version,
           NEW.created_at, NEW.expires_at)
       IS DISTINCT FROM ROW(OLD.id, OLD.purpose, OLD.provider, OLD.client_type, OLD.device_name, OLD.redirect_target,
           OLD.initiator_secret_hash, OLD.launch_ticket_hash, OLD.oauth_state_hash, OLD.oidc_nonce_hash,
           OLD.code_challenge, OLD.initiator_account_id, OLD.initiator_session_id,
           OLD.initiator_authenticated_at, OLD.initiator_authentication_method, OLD.initiator_credential_version,
           OLD.created_at, OLD.expires_at) THEN
        RAISE EXCEPTION 'Federated attempt context is immutable' USING ERRCODE = '23514';
    END IF;
    IF OLD.authorization_started_at IS NOT NULL
       AND NEW.authorization_started_at IS DISTINCT FROM OLD.authorization_started_at THEN
        RAISE EXCEPTION 'Authorization start is immutable' USING ERRCODE = '23514';
    END IF;
    IF OLD.status = 'verified' AND
       ROW(NEW.provider_subject, NEW.provider_email, NEW.email_authoritative,
           NEW.identity_verified_at, NEW.identity_valid_until, NEW.completion_code_hash,
           NEW.completion_issued_at, NEW.completion_expires_at)
       IS DISTINCT FROM ROW(OLD.provider_subject, OLD.provider_email, OLD.email_authoritative,
           OLD.identity_verified_at, OLD.identity_valid_until, OLD.completion_code_hash,
           OLD.completion_issued_at, OLD.completion_expires_at) THEN
        RAISE EXCEPTION 'Verified identity is immutable' USING ERRCODE = '23514';
    END IF;
    IF NEW.status = 'consumed' AND NOT EXISTS (
        SELECT 1 FROM auth.oauth_identities
        WHERE id = NEW.result_identity_id AND account_id = NEW.result_account_id
          AND provider = NEW.provider AND provider_subject = NEW.provider_subject
    ) THEN
        RAISE EXCEPTION 'Final identity does not match proof' USING ERRCODE = '23514';
    END IF;
    IF NEW.status = 'consumed' AND NEW.purpose = 'login' AND NOT EXISTS (
        SELECT 1 FROM auth.sessions
        WHERE id = NEW.result_session_id AND account_id = NEW.result_account_id
          AND authentication_method = 'google' AND oauth_identity_id = NEW.result_identity_id
    ) THEN
        RAISE EXCEPTION 'Final session does not match identity' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_protect_federated_auth_attempt
BEFORE INSERT OR UPDATE ON auth.federated_auth_attempts
FOR EACH ROW EXECUTE FUNCTION auth.protect_federated_auth_attempt();
