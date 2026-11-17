use reqwest::Url;
use zeroize::Zeroizing;

use crate::federated_identity::FederatedIdentityError;

pub struct GoogleOidcConfig {
    pub(super) client_id: String,
    pub(super) client_secret: Zeroizing<String>,
    pub(super) callback: Url,
}

impl GoogleOidcConfig {
    pub fn new(
        client_id: String,
        client_secret: String,
        callback: &str,
    ) -> Result<Self, FederatedIdentityError> {
        let client_secret = Zeroizing::new(client_secret);
        let callback = Url::parse(callback).map_err(|_| FederatedIdentityError::Configuration)?;
        if !valid_client_id(&client_id)
            || !valid_client_secret(&client_secret)
            || !valid_callback_url(&callback)
        {
            return Err(FederatedIdentityError::Configuration);
        }

        Ok(Self {
            client_id,
            client_secret,
            callback,
        })
    }
}

fn valid_client_id(client_id: &str) -> bool {
    !client_id.is_empty()
        && client_id.len() <= 256
        && client_id.ends_with(".apps.googleusercontent.com")
        && client_id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"-._".contains(&byte))
}

fn valid_client_secret(client_secret: &str) -> bool {
    !client_secret.is_empty()
        && client_secret.len() <= 1024
        && !client_secret
            .chars()
            .any(|character| character.is_whitespace() || character.is_control())
}

fn valid_callback_url(callback: &Url) -> bool {
    callback.scheme() == "https"
        && callback.host_str().is_some()
        && callback.username().is_empty()
        && callback.password().is_none()
        && callback.query().is_none()
        && callback.fragment().is_none()
        && callback.path() == "/api/v1/auth/google/callback"
}
