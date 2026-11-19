use std::{collections::HashSet, time::Duration};

use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::DecodingKey;
use reqwest::{header::HeaderMap, Client, Url};
use serde::Deserialize;
use tokio::time::Instant;

use crate::federated_identity::FederatedIdentityError;

const REFRESH_INTERVAL: Duration = Duration::from_secs(30);
const MAX_CACHE_SECONDS: u64 = 3600;
const MAX_JWKS_BYTES: usize = 64 * 1024;
const MAX_SIGNING_KEYS: usize = 16;

pub(super) struct SigningKeyCache {
    keys: Vec<(String, DecodingKey)>,
    valid_until: Instant,
    refresh_after: Instant,
}

#[derive(Deserialize)]
struct KeySet {
    keys: Vec<SigningKey>,
}

#[derive(Deserialize)]
struct SigningKey {
    #[serde(rename = "kid")]
    key_id: String,
    #[serde(rename = "kty")]
    key_type: String,
    #[serde(rename = "alg")]
    algorithm: Option<String>,
    #[serde(rename = "use")]
    usage: Option<String>,
    #[serde(rename = "key_ops")]
    operations: Option<Vec<String>>,
    #[serde(rename = "n")]
    modulus: Option<String>,
    #[serde(rename = "e")]
    exponent: Option<String>,
}

impl SigningKeyCache {
    pub(super) fn new() -> Self {
        let now = Instant::now();
        Self {
            keys: Vec::new(),
            valid_until: now,
            refresh_after: now,
        }
    }

    pub(super) async fn key(
        &mut self,
        client: &Client,
        endpoint: &Url,
        key_id: &str,
    ) -> Result<DecodingKey, FederatedIdentityError> {
        let now = Instant::now();
        if now < self.valid_until {
            if let Some((_, key)) = self.keys.iter().find(|(kid, _)| kid == key_id) {
                return Ok(key.clone());
            }
        }
        if now < self.refresh_after {
            return Err(if now < self.valid_until {
                FederatedIdentityError::InvalidIdentity
            } else {
                FederatedIdentityError::Unavailable
            });
        }
        self.refresh(client, endpoint, now).await?;
        self.keys
            .iter()
            .find(|(cached_id, _)| cached_id == key_id)
            .map(|(_, key)| key.clone())
            .ok_or(FederatedIdentityError::InvalidIdentity)
    }

    async fn refresh(
        &mut self,
        client: &Client,
        endpoint: &Url,
        now: Instant,
    ) -> Result<(), FederatedIdentityError> {
        // Reserve the refresh budget before I/O so cancellation cannot bypass it.
        self.refresh_after = now + REFRESH_INTERVAL;
        let response = client
            .get(endpoint.clone())
            .send()
            .await
            .map_err(|failure| {
                tracing::warn!(
                    event = "google_provider_request_failed",
                    stage = "jwks",
                    timeout = failure.is_timeout(),
                    connect = failure.is_connect(),
                    "Google signing keys could not be retrieved"
                );
                FederatedIdentityError::Unavailable
            })?;
        if response.status() != reqwest::StatusCode::OK {
            return Err(FederatedIdentityError::Unavailable);
        }

        let lifetime = cache_lifetime(response.headers());
        let body = super::bounded_body(response, MAX_JWKS_BYTES).await?;
        let key_set: KeySet =
            serde_json::from_slice(&body).map_err(|_| FederatedIdentityError::Unavailable)?;
        let keys = decode_keys(key_set)?;
        self.keys = keys;
        self.valid_until = now + lifetime;
        Ok(())
    }
}

fn decode_keys(key_set: KeySet) -> Result<Vec<(String, DecodingKey)>, FederatedIdentityError> {
    if key_set.keys.is_empty() || key_set.keys.len() > MAX_SIGNING_KEYS {
        return Err(FederatedIdentityError::Unavailable);
    }
    let mut keys = Vec::new();
    let mut identifiers = HashSet::new();
    for key in key_set.keys {
        if key.key_id.is_empty()
            || key.key_id.len() > super::MAX_KEY_ID_BYTES
            || !identifiers.insert(key.key_id.clone())
        {
            return Err(FederatedIdentityError::Unavailable);
        }
        if !key.supports_signature_verification() {
            continue;
        }

        let (key_id, decoding_key) = key.into_decoding_key()?;
        keys.push((key_id, decoding_key));
    }
    if keys.is_empty() {
        return Err(FederatedIdentityError::Unavailable);
    }

    Ok(keys)
}

impl SigningKey {
    fn supports_signature_verification(&self) -> bool {
        self.key_type == "RSA"
            && self
                .algorithm
                .as_deref()
                .is_none_or(|algorithm| algorithm == "RS256")
            && self.usage.as_deref().is_none_or(|usage| usage == "sig")
            && self
                .operations
                .as_ref()
                .is_none_or(|operations| operations.iter().any(|operation| operation == "verify"))
    }

    fn into_decoding_key(self) -> Result<(String, DecodingKey), FederatedIdentityError> {
        let modulus = self.modulus.ok_or(FederatedIdentityError::Unavailable)?;
        let exponent = self.exponent.ok_or(FederatedIdentityError::Unavailable)?;
        let modulus_bytes = URL_SAFE_NO_PAD
            .decode(&modulus)
            .map_err(|_| FederatedIdentityError::Unavailable)?;
        let exponent_bytes = URL_SAFE_NO_PAD
            .decode(&exponent)
            .map_err(|_| FederatedIdentityError::Unavailable)?;
        if !(256..=512).contains(&modulus_bytes.len())
            || modulus_bytes[0] < 128
            || exponent_bytes.is_empty()
            || exponent_bytes.len() > 4
            || exponent_bytes[0] == 0
            || exponent_bytes.last().is_none_or(|byte| byte % 2 == 0)
            || (exponent_bytes.len() == 1 && exponent_bytes[0] < 3)
        {
            return Err(FederatedIdentityError::Unavailable);
        }

        let decoding = DecodingKey::from_rsa_components(&modulus, &exponent)
            .map_err(|_| FederatedIdentityError::Unavailable)?;
        Ok((self.key_id, decoding))
    }
}

fn cache_lifetime(headers: &HeaderMap) -> Duration {
    let age = headers
        .get(reqwest::header::AGE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(0);
    let Some(control) = headers
        .get(reqwest::header::CACHE_CONTROL)
        .and_then(|value| value.to_str().ok())
    else {
        return Duration::ZERO;
    };

    let mut seconds = None;
    for directive in control.split(',').map(str::trim) {
        if directive.eq_ignore_ascii_case("no-store") || directive.eq_ignore_ascii_case("no-cache")
        {
            return Duration::ZERO;
        }
        if let Some((name, value)) = directive.split_once('=') {
            if name.eq_ignore_ascii_case("max-age") {
                let parsed = value.trim_matches('"').parse::<u64>().unwrap_or(0);
                seconds = Some(seconds.map_or(parsed, |current: u64| current.min(parsed)));
            }
        }
    }
    Duration::from_secs(
        seconds
            .unwrap_or(0)
            .saturating_sub(age)
            .min(MAX_CACHE_SECONDS),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{extract::State, http::StatusCode, routing::get, Router};
    use serde_json::json;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    };

    fn rsa_signing_key() -> serde_json::Value {
        json!({
            "kid": "test-key",
            "kty": "RSA",
            "alg": "RS256",
            "use": "sig",
            "key_ops": ["verify"],
            "n": URL_SAFE_NO_PAD.encode([0xd5u8; 256]),
            "e": "AQAB"
        })
    }

    #[test]
    fn unsupported_signing_keys_are_skipped_but_identifiers_must_be_unique() {
        for (field, value) in [
            ("kty", json!("EC")),
            ("alg", json!("RS512")),
            ("use", json!("enc")),
            ("key_ops", json!(["sign"])),
        ] {
            let mut unsupported = rsa_signing_key();
            unsupported["kid"] = json!("unsupported-key");
            unsupported[field] = value;
            let key_set = serde_json::from_value(json!({
                "keys": [unsupported, rsa_signing_key()]
            }))
            .unwrap();
            let decoded = decode_keys(key_set).unwrap();
            assert_eq!(decoded.len(), 1);
            assert_eq!(decoded[0].0, "test-key");
        }

        let mut unsupported = rsa_signing_key();
        unsupported["kty"] = json!("EC");
        let duplicate_ids = serde_json::from_value(json!({
            "keys": [unsupported, rsa_signing_key()]
        }))
        .unwrap();
        assert!(decode_keys(duplicate_ids).is_err());
    }

    #[test]
    fn malformed_rsa_components_are_rejected() {
        for (field, bytes) in [
            ("n", vec![]),
            ("n", vec![0xd5; 255]),
            ("n", vec![0xd5; 513]),
            ("n", vec![0x7f; 256]),
            ("e", vec![]),
            ("e", vec![0, 1, 1]),
            ("e", vec![2]),
            ("e", vec![1]),
            ("e", vec![1, 0, 0, 0, 1]),
        ] {
            let mut key = rsa_signing_key();
            key[field] = json!(URL_SAFE_NO_PAD.encode(bytes));
            let key_set = serde_json::from_value(json!({"keys": [key]})).unwrap();
            assert!(decode_keys(key_set).is_err(), "accepted invalid {field}");
        }
    }

    #[test]
    fn cache_lifetime_honors_age_directives_and_local_maximum() {
        for (control, age, seconds) in [
            ("public, max-age=120", "20", 100),
            ("max-age=86400", "0", 3600),
            ("max-age=30", "40", 0),
            ("max-age=120, no-store", "0", 0),
            ("max-age=120, no-cache", "0", 0),
            ("max-age=120, max-age=60", "0", 60),
            ("max-age=invalid", "0", 0),
            ("private", "0", 0),
        ] {
            let mut headers = HeaderMap::new();
            headers.insert(reqwest::header::CACHE_CONTROL, control.parse().unwrap());
            headers.insert(reqwest::header::AGE, age.parse().unwrap());
            assert_eq!(cache_lifetime(&headers).as_secs(), seconds);
        }
        assert_eq!(cache_lifetime(&HeaderMap::new()), Duration::ZERO);
    }

    #[tokio::test]
    async fn rotation_replaces_cached_keys_and_expired_keys_are_never_a_fallback() {
        struct ResponseState {
            response: Mutex<(StatusCode, String)>,
            requests: AtomicUsize,
        }
        fn jwks(kid: &str) -> String {
            json!({"keys":[{"kid":kid, "kty":"RSA", "alg":"RS256", "n":URL_SAFE_NO_PAD.encode([0xd5u8; 256]), "e":"AQAB"}]}).to_string()
        }

        let state = Arc::new(ResponseState {
            response: Mutex::new((StatusCode::OK, jwks("old-key"))),
            requests: AtomicUsize::new(0),
        });
        let router = Router::new()
            .route(
                "/keys",
                get(|State(state): State<Arc<ResponseState>>| async move {
                    state.requests.fetch_add(1, Ordering::SeqCst);
                    let (status, body) = state.response.lock().unwrap().clone();
                    (status, [("cache-control", "max-age=3600")], body)
                }),
            )
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint =
            Url::parse(&format!("http://{}/keys", listener.local_addr().unwrap())).unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let client = Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        let mut cache = SigningKeyCache::new();
        assert!(cache.key(&client, &endpoint, "old-key").await.is_ok());
        cache.refresh_after = Instant::now();
        *state.response.lock().unwrap() = (StatusCode::OK, jwks("new-key"));
        assert!(cache.key(&client, &endpoint, "new-key").await.is_ok());
        assert!(matches!(
            cache.key(&client, &endpoint, "old-key").await,
            Err(FederatedIdentityError::InvalidIdentity)
        ));
        assert_eq!(state.requests.load(Ordering::SeqCst), 2);
        cache.valid_until = Instant::now();
        assert!(matches!(
            cache.key(&client, &endpoint, "new-key").await,
            Err(FederatedIdentityError::Unavailable)
        ));
        cache.refresh_after = Instant::now();
        *state.response.lock().unwrap() = (StatusCode::SERVICE_UNAVAILABLE, "unavailable".into());
        assert!(matches!(
            cache.key(&client, &endpoint, "new-key").await,
            Err(FederatedIdentityError::Unavailable)
        ));
        assert_eq!(state.requests.load(Ordering::SeqCst), 3);
        task.abort();
    }
}
