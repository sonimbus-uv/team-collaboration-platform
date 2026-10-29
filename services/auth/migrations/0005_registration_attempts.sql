-- Independent pending registrations; existing accounts and legacy tokens are untouched.
CREATE TABLE auth.registration_attempts (
    id UUID PRIMARY KEY,
    email CITEXT NOT NULL,
    password_hash TEXT NOT NULL,
    code_mac BYTEA NOT NULL CHECK (octet_length(code_mac) = 32),
    code_version INTEGER NOT NULL DEFAULT 0 CHECK (code_version BETWEEN 0 AND 3),
    failed_attempts INTEGER NOT NULL DEFAULT 0 CHECK (failed_attempts BETWEEN 0 AND 5),
    created_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp() + interval '10 minutes',
    last_sent_at TIMESTAMPTZ NOT NULL DEFAULT statement_timestamp(),
    consumed_at TIMESTAMPTZ,
    invalidated_at TIMESTAMPTZ,
    CHECK (expires_at > created_at)
);

CREATE INDEX registration_attempts_pending_email_idx
    ON auth.registration_attempts(email, expires_at)
    WHERE consumed_at IS NULL AND invalidated_at IS NULL;
