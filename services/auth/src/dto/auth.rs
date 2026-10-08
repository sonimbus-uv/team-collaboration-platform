use crate::repositories::accounts_repository::Account;
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    pub user_id: String,
    pub email: String,
    pub email_verified: bool,
    pub status: String,
    pub verification_email_sent: bool,
}

impl From<Account> for RegisterResponse {
    fn from(account: Account) -> Self {
        Self {
            user_id: account.id.to_string(),
            email: account.email,
            email_verified: account.email_verified,
            status: account.status.to_string(),
            verification_email_sent: false,
        }
    }
}

impl std::fmt::Debug for RegisterRequest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RegisterRequest")
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn response_uses_persisted_verification() {
        for verified in [false, true] {
            let response = RegisterResponse::from(Account {
                id: sqlx::types::Uuid::nil(),
                email: "a@b.com".into(),
                status: "active".into(),
                email_verified: verified,
            });
            assert_eq!(response.email_verified, verified);
            assert_eq!(response.user_id, sqlx::types::Uuid::nil().to_string());
        }
    }
    #[test]
    fn request_debug_redacts_password() {
        let request = RegisterRequest {
            email: "a@b.com".into(),
            password: "secret".into(),
        };
        assert!(!format!("{request:?}").contains("secret"));
    }
}

#[derive(Deserialize)]
pub struct VerifyEmailRequest {
    pub token: String,
}

#[derive(Deserialize)]
pub struct ResendVerificationRequest {
    pub email: String,
}
