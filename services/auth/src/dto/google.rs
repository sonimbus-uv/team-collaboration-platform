use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::sessions::ClientType;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CreateGoogleAttempt {
    pub client_type: ClientType,
    #[serde(default, deserialize_with = "present_device")]
    pub device_name: Option<String>,
    pub redirect_target: String,
    pub code_challenge: String,
}

fn present_device<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    String::deserialize(deserializer).map(Some)
}

#[derive(Serialize)]
pub struct GoogleAttemptResponse {
    pub attempt_id: Uuid,
    pub attempt_secret: String,
    pub authorization_url: String,
    pub expires_at: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExchangeGoogleAttempt {
    pub attempt_id: Uuid,
    pub completion_code: String,
    pub code_verifier: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinkGoogleAttempt {
    pub redirect_target: String,
    pub code_challenge: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Reauthenticate {
    pub password: String,
}

#[derive(Serialize)]
pub struct GoogleConnection {
    pub provider: String,
    pub provider_email: Option<String>,
    pub linked_at: String,
}

#[derive(Serialize)]
pub struct GoogleConnections {
    pub connections: Vec<GoogleConnection>,
}
