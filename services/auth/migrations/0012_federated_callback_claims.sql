CREATE TABLE auth.federated_callback_claims (
    attempt_id UUID PRIMARY KEY REFERENCES auth.federated_auth_attempts(id) ON DELETE CASCADE,
    claimed_at TIMESTAMPTZ NOT NULL DEFAULT clock_timestamp()
);
