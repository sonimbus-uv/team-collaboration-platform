use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ChangePassword {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ForgotPassword {
    pub email: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResetPassword {
    pub recovery_attempt_id: Uuid,
    pub code: String,
    pub new_password: String,
}

#[derive(Serialize)]
pub struct RecoveryAccepted {
    pub recovery_attempt_id: Uuid,
    pub expires_at: String,
    pub message: &'static str,
}
