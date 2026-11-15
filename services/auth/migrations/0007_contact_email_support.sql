ALTER TABLE auth.accounts
    ADD COLUMN contact_email_version BIGINT NOT NULL DEFAULT 0,
    ADD CONSTRAINT accounts_contact_email_version_chk CHECK (contact_email_version >= 0);

CREATE FUNCTION auth.protect_contact_email_version()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF NEW.contact_email_version < OLD.contact_email_version THEN
        RAISE EXCEPTION 'Contact version cannot decrease' USING ERRCODE = '23514';
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_contact_email_version
BEFORE UPDATE OF contact_email_version ON auth.accounts
FOR EACH ROW EXECUTE FUNCTION auth.protect_contact_email_version();

CREATE TABLE auth.account_email_proposals (
    account_id UUID PRIMARY KEY REFERENCES auth.accounts(id) ON DELETE CASCADE,
    email CITEXT NOT NULL CHECK (length(btrim(email::text)) > 0),
    version BIGINT NOT NULL CHECK (version > 0),
    created_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    CHECK (updated_at >= created_at)
);

CREATE TABLE auth.contact_email_verification_attempts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    account_id UUID NOT NULL REFERENCES auth.accounts(id) ON DELETE CASCADE,
    email CITEXT NOT NULL CHECK (length(btrim(email::text)) > 0),
    contact_email_version BIGINT NOT NULL CHECK (contact_email_version > 0),
    code_mac BYTEA NOT NULL CHECK (octet_length(code_mac) = 32),
    code_version INTEGER NOT NULL DEFAULT 0 CHECK (code_version BETWEEN 0 AND 3),
    failed_attempts INTEGER NOT NULL DEFAULT 0 CHECK (failed_attempts BETWEEN 0 AND 5),
    created_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp() + interval '10 minutes',
    last_sent_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    consumed_at TIMESTAMPTZ,
    invalidated_at TIMESTAMPTZ,
    CHECK (expires_at > created_at AND expires_at <= created_at + interval '10 minutes'),
    CHECK (last_sent_at >= created_at AND last_sent_at < expires_at),
    CHECK (consumed_at IS NULL OR (consumed_at >= created_at AND consumed_at <= expires_at)),
    CHECK (invalidated_at IS NULL OR invalidated_at >= created_at),
    CHECK (consumed_at IS NULL OR invalidated_at IS NULL)
);

CREATE UNIQUE INDEX contact_email_one_pending_attempt
    ON auth.contact_email_verification_attempts(account_id)
    WHERE consumed_at IS NULL AND invalidated_at IS NULL;
CREATE INDEX contact_email_attempt_cleanup
    ON auth.contact_email_verification_attempts(expires_at);

-- Callers lock the account before modifying a proposal or its attempts.
CREATE FUNCTION auth.version_contact_email_proposal()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
DECLARE
    v_account_id UUID;
    v_version BIGINT;
BEGIN
    IF TG_OP <> 'DELETE' THEN
        NEW.email := lower(btrim(NEW.email::text))::citext;
    END IF;
    IF TG_OP = 'UPDATE' AND NEW.account_id IS DISTINCT FROM OLD.account_id THEN
        RAISE EXCEPTION 'Proposal account is immutable' USING ERRCODE = '23514';
    END IF;
    IF TG_OP = 'UPDATE' AND NEW.email IS NOT DISTINCT FROM OLD.email THEN
        IF ROW(NEW.version, NEW.created_at, NEW.updated_at)
           IS DISTINCT FROM ROW(OLD.version, OLD.created_at, OLD.updated_at) THEN
            RAISE EXCEPTION 'Proposal metadata is server-managed' USING ERRCODE = '23514';
        END IF;
        RETURN NEW;
    END IF;
    v_account_id := CASE WHEN TG_OP = 'DELETE' THEN OLD.account_id ELSE NEW.account_id END;
    UPDATE auth.accounts SET contact_email_version = contact_email_version + 1
    WHERE id = v_account_id RETURNING contact_email_version INTO v_version;

    UPDATE auth.contact_email_verification_attempts
       SET invalidated_at = clock_timestamp()
     WHERE account_id = v_account_id AND consumed_at IS NULL AND invalidated_at IS NULL;

    IF TG_OP = 'DELETE' THEN
        RETURN OLD;
    END IF;
    IF v_version IS NULL THEN
        RAISE EXCEPTION 'Proposal account does not exist' USING ERRCODE = '23503';
    END IF;
    NEW.version := v_version;
    NEW.updated_at := clock_timestamp();
    IF TG_OP = 'UPDATE' THEN
        NEW.created_at := OLD.created_at;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_version_contact_email_proposal
BEFORE INSERT OR UPDATE OR DELETE ON auth.account_email_proposals
FOR EACH ROW EXECUTE FUNCTION auth.version_contact_email_proposal();

CREATE FUNCTION auth.protect_contact_email_attempt()
RETURNS TRIGGER LANGUAGE plpgsql AS $$
BEGIN
    IF TG_OP = 'INSERT' THEN
        PERFORM id FROM auth.accounts WHERE id = NEW.account_id FOR UPDATE;
        IF NOT EXISTS (
            SELECT 1 FROM auth.account_email_proposals
            WHERE account_id = NEW.account_id AND email = NEW.email
              AND version = NEW.contact_email_version
        ) THEN
            RAISE EXCEPTION 'Contact proposal does not match' USING ERRCODE = '23514';
        END IF;
    ELSE
        IF OLD.consumed_at IS NULL AND NEW.consumed_at IS NOT NULL
           AND (OLD.failed_attempts >= 5 OR OLD.expires_at <= clock_timestamp()) THEN
            RAISE EXCEPTION 'Contact attempt is not consumable' USING ERRCODE = '23514';
        END IF;
        IF (OLD.consumed_at IS NOT NULL OR OLD.invalidated_at IS NOT NULL)
           AND NEW IS DISTINCT FROM OLD THEN
            RAISE EXCEPTION 'Terminal contact attempt is immutable' USING ERRCODE = '23514';
        END IF;
        IF ROW(NEW.id, NEW.account_id, NEW.email, NEW.contact_email_version, NEW.created_at, NEW.expires_at)
           IS DISTINCT FROM ROW(OLD.id, OLD.account_id, OLD.email, OLD.contact_email_version, OLD.created_at, OLD.expires_at)
           OR NEW.failed_attempts < OLD.failed_attempts THEN
            RAISE EXCEPTION 'Contact attempt context is immutable' USING ERRCODE = '23514';
        END IF;
        IF NEW.code_version = OLD.code_version THEN
            IF ROW(NEW.code_mac, NEW.last_sent_at) IS DISTINCT FROM ROW(OLD.code_mac, OLD.last_sent_at) THEN
                RAISE EXCEPTION 'Code replacement requires a new version' USING ERRCODE = '23514';
            END IF;
        ELSIF NEW.code_version <> OLD.code_version + 1
           OR NEW.code_mac = OLD.code_mac
           OR NEW.last_sent_at < OLD.last_sent_at + interval '60 seconds'
           OR OLD.failed_attempts >= 5 THEN
            RAISE EXCEPTION 'Invalid contact code replacement' USING ERRCODE = '23514';
        END IF;
    END IF;
    RETURN NEW;
END;
$$;

CREATE TRIGGER trg_protect_contact_email_attempt
BEFORE INSERT OR UPDATE ON auth.contact_email_verification_attempts
FOR EACH ROW EXECUTE FUNCTION auth.protect_contact_email_attempt();
