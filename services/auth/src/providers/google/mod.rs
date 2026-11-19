mod claims;
pub mod config;
mod keys;
#[cfg(test)]
mod tests;

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use jsonwebtoken::{decode_header, Algorithm};
use reqwest::{Client, Url};
use serde::Deserialize;
use tokio::sync::Mutex;
use zeroize::{Zeroize, Zeroizing};

use crate::federated_identity::{
    AuthorizationCode, AuthorizationRequest, FederatedIdentityError, FederatedIdentityProvider,
    VerifiedIdentity,
};
use config::GoogleOidcConfig;
use keys::SigningKeyCache;

const ISSUER: &str = "https://accounts.google.com";
const AUTHORIZATION_ENDPOINT: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_ENDPOINT: &str = "https://oauth2.googleapis.com/token";
const JWKS_ENDPOINT: &str = "https://www.googleapis.com/oauth2/v3/certs";
const TOTAL_TIMEOUT: Duration = Duration::from_secs(12);
const CONNECT_TIMEOUT: Duration = Duration::from_secs(5);
const REQUEST_TIMEOUT: Duration = Duration::from_secs(8);
const MAX_AUTHORIZATION_CODE_BYTES: usize = 4096;
const MAX_ID_TOKEN_BYTES: usize = 16384;
const MAX_KEY_ID_BYTES: usize = 256;
const MAX_TOKEN_RESPONSE: usize = 32 * 1024;

pub struct GoogleOidcProvider {
    config: GoogleOidcConfig,
    client: Client,
    signing_keys: Mutex<SigningKeyCache>,
    token_endpoint: Url,
    jwks_endpoint: Url,
}

#[derive(Deserialize)]
struct TokenResponse {
    id_token: String,
}

impl Drop for TokenResponse {
    fn drop(&mut self) {
        self.id_token.zeroize();
    }
}

impl GoogleOidcProvider {
    pub fn new(config: GoogleOidcConfig) -> Result<Self, FederatedIdentityError> {
        let client = Client::builder()
            .https_only(true)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(CONNECT_TIMEOUT)
            .timeout(REQUEST_TIMEOUT)
            .build()
            .map_err(|_| FederatedIdentityError::Configuration)?;
        Ok(Self {
            config,
            client,
            signing_keys: Mutex::new(SigningKeyCache::new()),
            token_endpoint: Url::parse(TOKEN_ENDPOINT)
                .map_err(|_| FederatedIdentityError::Configuration)?,
            jwks_endpoint: Url::parse(JWKS_ENDPOINT)
                .map_err(|_| FederatedIdentityError::Configuration)?,
        })
    }

    async fn exchange(
        &self,
        authorization: AuthorizationCode<'_>,
    ) -> Result<VerifiedIdentity, FederatedIdentityError> {
        if authorization.code.is_empty()
            || authorization.code.len() > MAX_AUTHORIZATION_CODE_BYTES
            || authorization.code.chars().any(char::is_control)
        {
            return Err(FederatedIdentityError::InvalidRequest);
        }

        let token = self.request_identity_token(authorization.code).await?;
        let key_id = identity_token_key_id(&token.id_token)?;
        let key = self
            .signing_keys
            .lock()
            .await
            .key(&self.client, &self.jwks_endpoint, &key_id)
            .await?;
        claims::verify(
            &token.id_token,
            &key,
            &self.config.client_id,
            authorization.expected_nonce_hash,
        )
    }

    async fn request_identity_token(
        &self,
        code: &str,
    ) -> Result<TokenResponse, FederatedIdentityError> {
        let response = self
            .client
            .post(self.token_endpoint.clone())
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("client_id", self.config.client_id.as_str()),
                ("client_secret", self.config.client_secret.as_str()),
                ("redirect_uri", self.config.callback.as_str()),
            ])
            .send()
            .await
            .map_err(|failure| {
                tracing::warn!(
                    event = "google_provider_request_failed",
                    stage = "token",
                    timeout = failure.is_timeout(),
                    connect = failure.is_connect(),
                    "Google token endpoint could not be reached"
                );
                FederatedIdentityError::Unavailable
            })?;
        if response.status() != reqwest::StatusCode::OK {
            tracing::warn!(
                event = "google_provider_response_rejected",
                stage = "token",
                http_status = response.status().as_u16(),
                "Google token endpoint rejected the request"
            );
        }
        if response.status() == reqwest::StatusCode::BAD_REQUEST {
            return Err(FederatedIdentityError::AuthorizationRejected);
        }
        if response.status() != reqwest::StatusCode::OK {
            return Err(FederatedIdentityError::Unavailable);
        }

        let body = bounded_body(response, MAX_TOKEN_RESPONSE).await?;
        let token: TokenResponse =
            serde_json::from_slice(&body).map_err(|_| FederatedIdentityError::InvalidIdentity)?;
        Ok(token)
    }
}

#[async_trait::async_trait]
impl FederatedIdentityProvider for GoogleOidcProvider {
    fn authorization_url(
        &self,
        request: AuthorizationRequest<'_>,
    ) -> Result<String, FederatedIdentityError> {
        if !valid_correlation_value(request.state) || !valid_correlation_value(request.nonce) {
            return Err(FederatedIdentityError::InvalidRequest);
        }

        let mut url = Url::parse(AUTHORIZATION_ENDPOINT)
            .map_err(|_| FederatedIdentityError::Configuration)?;
        url.query_pairs_mut().extend_pairs([
            ("client_id", self.config.client_id.as_str()),
            ("redirect_uri", self.config.callback.as_str()),
            ("response_type", "code"),
            ("scope", "openid email"),
            ("state", request.state),
            ("nonce", request.nonce),
            ("prompt", "select_account"),
        ]);
        Ok(url.into())
    }

    async fn exchange_code(
        &self,
        authorization: AuthorizationCode<'_>,
    ) -> Result<VerifiedIdentity, FederatedIdentityError> {
        tokio::time::timeout(TOTAL_TIMEOUT, self.exchange(authorization))
            .await
            .map_err(|_| FederatedIdentityError::Unavailable)?
    }
}

fn identity_token_key_id(token: &str) -> Result<String, FederatedIdentityError> {
    if token.len() > MAX_ID_TOKEN_BYTES {
        return Err(FederatedIdentityError::InvalidIdentity);
    }

    let header = decode_header(token).map_err(|_| FederatedIdentityError::InvalidIdentity)?;
    if header.alg != Algorithm::RS256
        || header.crit.is_some()
        || header.jku.is_some()
        || header.jwk.is_some()
        || header.x5u.is_some()
    {
        return Err(FederatedIdentityError::InvalidIdentity);
    }

    header
        .kid
        .filter(|key_id| !key_id.is_empty() && key_id.len() <= MAX_KEY_ID_BYTES)
        .ok_or(FederatedIdentityError::InvalidIdentity)
}

fn valid_correlation_value(value: &str) -> bool {
    (43..=256).contains(&value.len())
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
}

fn unix_time() -> Result<u64, FederatedIdentityError> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|time| time.as_secs())
        .map_err(|_| FederatedIdentityError::Unavailable)
}

async fn bounded_body(
    mut response: reqwest::Response,
    maximum: usize,
) -> Result<Zeroizing<Vec<u8>>, FederatedIdentityError> {
    if response
        .content_length()
        .is_some_and(|length| length > maximum as u64)
    {
        return Err(FederatedIdentityError::Unavailable);
    }
    let mut body = Zeroizing::new(Vec::new());
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|_| FederatedIdentityError::Unavailable)?
    {
        if chunk.len() > maximum.saturating_sub(body.len()) {
            return Err(FederatedIdentityError::Unavailable);
        }
        body.extend_from_slice(&chunk);
    }

    Ok(body)
}
