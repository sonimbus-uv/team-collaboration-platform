use anyhow::ensure;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

pub const MAX_ACCESS_SECONDS: u64 = 900;
pub const MAX_SESSION_SECONDS: u64 = 20 * 24 * 60 * 60;

/// Session context must come from authenticated server state, not client claims.
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

        let max_session_expires_at = self
            .login_at
            .checked_add(MAX_SESSION_SECONDS)
            .ok_or_else(|| anyhow::anyhow!("Session timestamp overflow"))?;

        ensure!(
            self.expires_at > self.login_at && self.expires_at <= max_session_expires_at,
            "Session exceeds absolute lifetime"
        );

        ensure!(self.expires_at > now, "Session has expired");

        let max_access_expires_at = now
            .checked_add(MAX_ACCESS_SECONDS)
            .ok_or_else(|| anyhow::anyhow!("Access timestamp overflow"))?;

        Ok(max_access_expires_at.min(self.expires_at))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn access_expiration_accepts_exact_limits_and_caps_at_session_expiry() {
        let login_at = 1_000;
        let mut session = SessionContext {
            account_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            login_at,
            expires_at: login_at + MAX_SESSION_SECONDS,
        };
        assert_eq!(
            session.access_expiration(login_at).unwrap(),
            login_at + MAX_ACCESS_SECONDS
        );
        assert_eq!(
            session.access_expiration(session.expires_at - 1).unwrap(),
            session.expires_at
        );
        assert!(session.access_expiration(session.expires_at).is_err());
        session.expires_at += 1;
        assert!(session.access_expiration(login_at).is_err());
    }

    #[test]
    fn invalid_login_times_and_timestamp_overflows_return_errors() {
        let mut session = SessionContext {
            account_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            login_at: 0,
            expires_at: 100,
        };
        assert!(session.access_expiration(1).is_err());
        session.login_at = 2;
        assert!(session.access_expiration(1).is_err());
        session.login_at = u64::MAX - 1;
        session.expires_at = u64::MAX;
        assert!(session.access_expiration(u64::MAX - 1).is_err());
        session.login_at = u64::MAX - MAX_SESSION_SECONDS;
        assert!(session.access_expiration(u64::MAX - 1).is_err());
    }
}
