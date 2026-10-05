-- =====================================================================
-- Authentication service - auth schema
-- Requires PostgreSQL 13+ (gen_random_uuid() is built in; pgcrypto is not required)
-- =====================================================================

CREATE EXTENSION IF NOT EXISTS citext;

CREATE SCHEMA IF NOT EXISTS auth;

-- ---------------------------------------------------------------------
-- Automatic updated_at
-- The database owns this timestamp regardless of whether writes come from the
-- ORM, a query builder, a migration, or a manual query. The WHEN clause avoids
-- changing updated_at on UPDATE statements that do not actually change data.
-- ---------------------------------------------------------------------
CREATE OR REPLACE FUNCTION auth.set_updated_at()
RETURNS TRIGGER
LANGUAGE plpgsql
AS $$
BEGIN
    NEW.updated_at := now();
    RETURN NEW;
END;
$$;

-- ---------------------------------------------------------------------
-- Accounts
-- Pure identity data: no profile fields and no authentication method.
-- users.id in the application domain equals auth.accounts.id, without a
-- cross-schema foreign key.
-- Verification is represented by email_verified_at IS NOT NULL.
-- status only represents the administrative lifecycle.
-- ---------------------------------------------------------------------
CREATE TABLE auth.accounts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    email CITEXT NOT NULL,
    email_verified_at TIMESTAMPTZ,

    status VARCHAR(20) NOT NULL DEFAULT 'active',

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_login_at TIMESTAMPTZ,

    CONSTRAINT accounts_email_unique
        UNIQUE (email),

    CONSTRAINT accounts_status_chk
        CHECK (status IN ('active', 'locked', 'suspended', 'deleted'))
);

CREATE TRIGGER trg_accounts_updated_at
BEFORE UPDATE ON auth.accounts
FOR EACH ROW
WHEN (OLD.* IS DISTINCT FROM NEW.*)
EXECUTE FUNCTION auth.set_updated_at();

-- ---------------------------------------------------------------------
-- Password credentials
-- A row exists if and only if the account can sign in with a password.
-- The rule "every account has at least one credential" is enforced by the
-- service because it cannot be expressed with a cross-table CHECK constraint.
-- ---------------------------------------------------------------------
CREATE TABLE auth.password_credentials (
    account_id UUID PRIMARY KEY
        REFERENCES auth.accounts(id)
        ON DELETE CASCADE,

    password_hash TEXT NOT NULL,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    password_changed_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- ---------------------------------------------------------------------
-- OAuth identities
-- A row exists if and only if the account can sign in with that provider.
-- provider_email is nullable because some providers do not return an email
-- address or return a relay address.
-- ---------------------------------------------------------------------
CREATE TABLE auth.oauth_identities (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    account_id UUID NOT NULL
        REFERENCES auth.accounts(id)
        ON DELETE CASCADE,

    provider VARCHAR(30) NOT NULL,
    provider_subject TEXT NOT NULL,
    provider_email CITEXT,

    linked_at TIMESTAMPTZ NOT NULL DEFAULT now(),

    CONSTRAINT oauth_provider_subject_unique
        UNIQUE (provider, provider_subject),

    CONSTRAINT oauth_account_provider_unique
        UNIQUE (account_id, provider)
);

-- ---------------------------------------------------------------------
-- Sessions
-- Only one active session is allowed per (account, client type).
-- Opening a session must always go through auth.open_session(), which revokes
-- the previous session before inserting the new one. This prevents an expired
-- but not-yet-revoked session from blocking the next login.
-- ---------------------------------------------------------------------
CREATE TABLE auth.sessions (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    account_id UUID NOT NULL
        REFERENCES auth.accounts(id)
        ON DELETE CASCADE,

    client_type VARCHAR(30) NOT NULL,

    device_name VARCHAR(120),
    ip_address INET,
    user_agent TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    last_seen_at TIMESTAMPTZ,
    expires_at TIMESTAMPTZ NOT NULL,

    revoked_at TIMESTAMPTZ,
    revoke_reason VARCHAR(30),

    CONSTRAINT sessions_client_type_chk
        CHECK (client_type IN ('mobile', 'desktop')),

    CONSTRAINT sessions_expiry_chk
        CHECK (expires_at > created_at),

    CONSTRAINT sessions_revocation_consistency_chk
        CHECK ((revoked_at IS NULL) = (revoke_reason IS NULL)),

    CONSTRAINT sessions_revoke_reason_chk
        CHECK (revoke_reason IN (
            'logout',            -- the user signed out
            'superseded',        -- replaced by a new login from the same client
            'expired',           -- expired and marked during replacement or cleanup
            'refresh_reuse',     -- refresh token reuse was detected
            'password_changed',  -- password change or password reset
            'admin'              -- revoked by an administrator
        ))
);

CREATE UNIQUE INDEX ux_active_session_per_client
ON auth.sessions (account_id, client_type)
WHERE revoked_at IS NULL;

CREATE INDEX idx_sessions_expires_at
ON auth.sessions (expires_at);

-- Opens a session by replacing any active session from the same client.
-- The advisory lock serializes concurrent logins for the same (account, client)
-- pair. Without it, two simultaneous logins could fail on the unique index
-- instead of following a "last login wins" rule.
CREATE OR REPLACE FUNCTION auth.open_session(
    p_account_id  UUID,
    p_client_type VARCHAR,
    p_ttl         INTERVAL,
    p_device_name VARCHAR DEFAULT NULL,
    p_ip_address  INET    DEFAULT NULL,
    p_user_agent  TEXT    DEFAULT NULL
)
RETURNS UUID
LANGUAGE plpgsql
AS $$
DECLARE
    v_session_id UUID;
BEGIN
    PERFORM pg_advisory_xact_lock(
        hashtextextended(p_account_id::text || ':' || p_client_type, 0)
    );

    UPDATE auth.sessions
       SET revoked_at    = now(),
           revoke_reason = CASE
                               WHEN expires_at <= now() THEN 'expired'
                               ELSE 'superseded'
                           END
     WHERE account_id  = p_account_id
       AND client_type = p_client_type
       AND revoked_at IS NULL;

    INSERT INTO auth.sessions (
        account_id, client_type, device_name,
        ip_address, user_agent, last_seen_at, expires_at
    )
    VALUES (
        p_account_id, p_client_type, p_device_name,
        p_ip_address, p_user_agent, now(), now() + p_ttl
    )
    RETURNING id INTO v_session_id;

    RETURN v_session_id;
END;
$$;

-- ---------------------------------------------------------------------
-- Refresh tokens
-- The session is the token family: if an already-rotated token is reused, the
-- whole session is revoked with refresh_reuse.
-- When validating a refresh token, the service must also verify that the
-- session is not revoked or expired.
-- ---------------------------------------------------------------------
CREATE TABLE auth.refresh_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    session_id UUID NOT NULL
        REFERENCES auth.sessions(id)
        ON DELETE CASCADE,

    token_hash BYTEA NOT NULL UNIQUE,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    revoked_at TIMESTAMPTZ,

    replaced_by_token_id UUID
        REFERENCES auth.refresh_tokens(id)
        ON DELETE SET NULL
);

CREATE INDEX idx_refresh_tokens_session_id
ON auth.refresh_tokens (session_id);

CREATE INDEX idx_refresh_tokens_expires_at
ON auth.refresh_tokens (expires_at);

-- ---------------------------------------------------------------------
-- One-time tokens
-- Email verification, email change, and password reset tokens are represented
-- in a single table using purpose.
-- When issuing a new token, the service invalidates pending tokens for the
-- same (account, purpose) in the same transaction.
-- ---------------------------------------------------------------------
CREATE TABLE auth.one_time_tokens (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    account_id UUID NOT NULL
        REFERENCES auth.accounts(id)
        ON DELETE CASCADE,

    purpose VARCHAR(30) NOT NULL,

    token_hash BYTEA NOT NULL UNIQUE,

    sent_to_email CITEXT NOT NULL,

    requested_ip INET,
    requested_user_agent TEXT,

    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    expires_at TIMESTAMPTZ NOT NULL,
    used_at TIMESTAMPTZ,
    invalidated_at TIMESTAMPTZ,

    CONSTRAINT one_time_tokens_purpose_chk
        CHECK (purpose IN ('email_verification', 'email_change', 'password_reset')),

    CONSTRAINT one_time_tokens_expiry_chk
        CHECK (expires_at > created_at),

    CONSTRAINT one_time_tokens_final_state_chk
        CHECK (used_at IS NULL OR invalidated_at IS NULL),

    -- Allows other tables to require a specific purpose through a composite FK.
    CONSTRAINT one_time_tokens_id_purpose_unique
        UNIQUE (id, purpose)
);

CREATE INDEX idx_one_time_tokens_pending
ON auth.one_time_tokens (account_id, purpose)
WHERE used_at IS NULL AND invalidated_at IS NULL;

CREATE INDEX idx_one_time_tokens_expires_at
ON auth.one_time_tokens (expires_at);

-- ---------------------------------------------------------------------
-- Email change requests
-- The composite FK guarantees that the associated token has purpose
-- 'email_change' instead of email verification or password reset.
-- Only one pending request is allowed per account. When creating a new one, the
-- service cancels the pending request in the same transaction, following the
-- same pattern as sessions. This prevents a request with an expired token from
-- blocking the next request.
-- ---------------------------------------------------------------------
CREATE TABLE auth.email_change_requests (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    account_id UUID NOT NULL
        REFERENCES auth.accounts(id)
        ON DELETE CASCADE,

    current_email CITEXT NOT NULL,
    requested_email CITEXT NOT NULL,

    verification_token_id UUID NOT NULL,
    token_purpose VARCHAR(30) NOT NULL DEFAULT 'email_change',

    requested_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    confirmed_at TIMESTAMPTZ,
    cancelled_at TIMESTAMPTZ,

    CONSTRAINT email_change_different_email_chk
        CHECK (current_email <> requested_email),

    CONSTRAINT email_change_token_purpose_chk
        CHECK (token_purpose = 'email_change'),

    CONSTRAINT email_change_final_state_chk
        CHECK (confirmed_at IS NULL OR cancelled_at IS NULL),

    CONSTRAINT email_change_token_fk
        FOREIGN KEY (verification_token_id, token_purpose)
        REFERENCES auth.one_time_tokens (id, purpose)
        ON DELETE CASCADE
);

CREATE INDEX idx_email_change_requests_account_id
ON auth.email_change_requests (account_id);

CREATE UNIQUE INDEX ux_pending_email_change_per_account
ON auth.email_change_requests (account_id)
WHERE confirmed_at IS NULL AND cancelled_at IS NULL;
