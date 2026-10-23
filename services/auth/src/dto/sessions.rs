use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClientType {
    Desktop,
    Mobile,
}

impl ClientType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Desktop => "desktop",
            Self::Mobile => "mobile",
        }
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
    pub client_type: ClientType,
    #[serde(default, deserialize_with = "present_device")]
    pub device_name: Option<String>,
}

fn present_device<'de, D: serde::Deserializer<'de>>(value: D) -> Result<Option<String>, D::Error> {
    String::deserialize(value).map(Some)
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RefreshRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: &'static str,
    pub access_expires_in: u64,
    pub session_expires_at: String,
    pub sid: Uuid,
    pub account_id: Uuid,
}

// Never derive Debug for credentials, including test failure paths.
macro_rules! redacted_debug {
    ($($ty:ty),+) => {$(impl std::fmt::Debug for $ty {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            f.write_str(concat!(stringify!($ty), "([REDACTED])"))
        }
    })+};
}
redacted_debug!(LoginRequest, RefreshRequest, TokenResponse);
