use std::collections::HashMap;

use auth_service::google_oidc::config::GoogleOidcConfig;
use reqwest::Url;
use serde::Deserialize;

use crate::dto::sessions::ClientType;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GoogleReturnTarget {
    pub client_type: ClientType,
    #[serde(deserialize_with = "deserialize_url")]
    pub url: Url,
}

pub struct GoogleAuthConfig {
    pub provider: GoogleOidcConfig,
    pub authorization_url: Url,
    pub targets: HashMap<String, GoogleReturnTarget>,
}

impl GoogleAuthConfig {
    pub fn new(
        client_id: String,
        client_secret: String,
        callback: &str,
        targets: &str,
    ) -> anyhow::Result<Self> {
        let provider = GoogleOidcConfig::new(client_id, client_secret, callback)?;
        let mut authorization_url =
            Url::parse(callback).map_err(|_| anyhow::anyhow!("Invalid Google callback"))?;
        authorization_url.set_path("/api/v1/auth/google/authorize");
        let targets: HashMap<String, GoogleReturnTarget> = serde_json::from_str(targets)
            .map_err(|_| anyhow::anyhow!("Invalid Google return targets"))?;
        anyhow::ensure!(
            !targets.is_empty() && targets.len() <= 16,
            "Invalid Google return targets"
        );
        for (name, target) in &targets {
            anyhow::ensure!(
                valid_target_name(name) && valid_return_url(&target.url),
                "Invalid Google return targets"
            );
        }

        Ok(Self {
            provider,
            authorization_url,
            targets,
        })
    }
}

fn valid_target_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-_".contains(&byte))
}

fn valid_return_url(url: &Url) -> bool {
    matches!(url.scheme(), "https" | "sonimbus")
        && url.host_str().is_some()
        && url.username().is_empty()
        && url.password().is_none()
        && url.query().is_none()
        && url.fragment().is_none()
}

fn deserialize_url<'de, D: serde::Deserializer<'de>>(deserializer: D) -> Result<Url, D::Error> {
    let value = String::deserialize(deserializer)?;
    Url::parse(&value).map_err(|_| serde::de::Error::custom("Invalid Google return target URL"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn return_targets_reject_arbitrary_urls_and_invalid_clients() {
        for target in [
            r#"{}"#,
            r#"{"desktop":{"client_type":"web","url":"sonimbus://auth/google"}}"#,
            r#"{"desktop":{"client_type":"desktop","url":"javascript:alert(1)"}}"#,
            r#"{"desktop":{"client_type":"desktop","url":"https://return.invalid/?next=evil"}}"#,
            r#"{"desktop":{"client_type":"desktop","url":"http://127.0.0.1/callback"}}"#,
        ] {
            assert!(GoogleAuthConfig::new(
                "test.apps.googleusercontent.com".into(),
                "test-secret".into(),
                "https://auth.example.invalid/api/v1/auth/google/callback",
                target
            )
            .is_err());
        }
    }
}
