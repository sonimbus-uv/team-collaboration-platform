use crate::repositories::accounts_repository::Account;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct RegistrationResponse {
    pub registration_attempt_id: Uuid,
    pub expires_at: String,
    pub verification_email_sent: bool,
}

#[derive(Serialize)]
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
            status: account.status,
            verification_email_sent: false,
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyEmailRequest {
    pub registration_attempt_id: Uuid,
    pub code: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResendVerificationRequest {
    pub registration_attempt_id: Uuid,
}
