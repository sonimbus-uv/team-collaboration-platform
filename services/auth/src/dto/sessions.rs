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
    #[serde(default, deserialize_with = "deserialize_present_device_name")]
    pub device_name: Option<String>,
}

fn deserialize_present_device_name<'de, D: serde::Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<String>, D::Error> {
    String::deserialize(deserializer).map(Some)
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
    ($($credential_type:ty),+) => {
        $(
            impl std::fmt::Debug for $credential_type {
                fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    formatter.write_str(concat!(stringify!($credential_type), "([REDACTED])"))
                }
            }
        )+
    };
}

redacted_debug!(LoginRequest, RefreshRequest, TokenResponse);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_name_may_be_absent_but_must_be_a_string_when_present() {
        let base = serde_json::json!({"email": "user@example.com", "password": "secret", "client_type": "desktop"});
        let absent: LoginRequest = serde_json::from_value(base.clone()).unwrap();
        assert!(absent.device_name.is_none());
        for invalid in [
            serde_json::Value::Null,
            serde_json::json!(42),
            serde_json::json!([]),
        ] {
            let mut request = base.clone();
            request["device_name"] = invalid;
            assert!(serde_json::from_value::<LoginRequest>(request).is_err());
        }
        let mut request = base;
        request["device_name"] = serde_json::json!("Work laptop");
        let present: LoginRequest = serde_json::from_value(request).unwrap();
        assert_eq!(present.device_name.as_deref(), Some("Work laptop"));
    }

    #[test]
    fn credential_debug_output_is_redacted() {
        let request = RefreshRequest {
            refresh_token: "private-refresh-token".into(),
        };
        assert_eq!(format!("{request:?}"), "RefreshRequest([REDACTED])");
    }
}
