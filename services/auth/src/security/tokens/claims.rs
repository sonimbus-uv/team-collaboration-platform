use anyhow::ensure;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_ACCESS_SECONDS: u64 = 900;
pub const MAX_SESSION_SECONDS: u64 = 20 * 24 * 60 * 60;

/// Supplied by the future authenticated session flow, never HTTP client claims.
pub struct SessionContext {
    pub account_id: Uuid,
    pub session_id: Uuid,
    pub login_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AccessClaims {
    pub iss: String,
    pub aud: Vec<String>,
    pub sub: Uuid,
    pub sid: Uuid,
    pub iat: u64,
    pub exp: u64,
    pub jti: Uuid,
}

impl SessionContext {
    pub(super) fn access_expiration(&self, now: u64) -> anyhow::Result<u64> {
        ensure!(
            !self.account_id.is_nil() && !self.session_id.is_nil(),
            "Account and session IDs must not be nil"
        );
        ensure!(
            self.login_at > 0 && self.login_at <= now,
            "Invalid session login timestamp"
        );

        let max_session = self
            .login_at
            .checked_add(MAX_SESSION_SECONDS)
            .ok_or_else(|| anyhow::anyhow!("Session timestamp overflow"))?;

        ensure!(
            self.expires_at > self.login_at && self.expires_at <= max_session,
            "Session exceeds absolute lifetime"
        );
        ensure!(self.expires_at > now, "Session has expired");

        let max_access = now
            .checked_add(MAX_ACCESS_SECONDS)
            .ok_or_else(|| anyhow::anyhow!("Access timestamp overflow"))?;

        Ok(max_access.min(self.expires_at))
    }
}
