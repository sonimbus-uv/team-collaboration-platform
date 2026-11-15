use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Serialize, sqlx::FromRow)]
pub struct ContactEmail {
    pub contact_email: Option<String>,
    pub contact_email_verified: bool,
    pub pending_contact_email: Option<String>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RequestContactVerification {
    pub email: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResendContactVerification {
    pub verification_attempt_id: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VerifyContactEmail {
    pub verification_attempt_id: Uuid,
    pub code: String,
}

#[derive(Serialize, sqlx::FromRow)]
pub struct ContactVerificationAccepted {
    pub verification_attempt_id: Uuid,
    pub expires_at: String,
}
