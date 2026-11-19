-- Recovery codes are purpose-bound MACs; plaintext codes are never persisted.
CREATE TABLE auth.password_recovery_attempts (
    id UUID PRIMARY KEY,
    account_id UUID NOT NULL REFERENCES auth.accounts(id),
    credential_version BIGINT NOT NULL,
    code_mac BYTEA NOT NULL CHECK (octet_length(code_mac) = 32),
    failed_attempts INTEGER NOT NULL DEFAULT 0 CHECK (failed_attempts BETWEEN 0 AND 5),
    created_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp(),
    expires_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp() + interval '10 minutes',
    consumed_at TIMESTAMPTZ,
    invalidated_at TIMESTAMPTZ,
    CHECK (expires_at > created_at)
);
CREATE UNIQUE INDEX password_recovery_one_pending
ON auth.password_recovery_attempts(account_id)
WHERE consumed_at IS NULL AND invalidated_at IS NULL;
CREATE INDEX password_recovery_cleanup ON auth.password_recovery_attempts(expires_at);
