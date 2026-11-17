use std::{collections::HashSet, path::Path};

use anyhow::ensure;
use axum::http::{header::AUTHORIZATION, HeaderMap};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use sqlx::types::chrono::{DateTime, Utc};
use subtle::ConstantTimeEq;
use zeroize::Zeroizing;

use super::config::read_file;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Identity {
    Core,
    Files,
}

impl Identity {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Files => "files",
        }
    }
}

// No Debug or Serialize for credential configuration or its in-memory snapshot.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialDocument {
    core: CredentialEntry,
    files: CredentialEntry,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct CredentialEntry {
    enabled: bool,
    permissions: Vec<String>,
    active_sha256: Option<String>,
    previous: Option<PreviousCredential>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct PreviousCredential {
    sha256: String,
    expires_at: String,
}

struct ServiceCredential {
    identity: Identity,
    enabled: bool,
    allowed: bool,
    active: Option<[u8; 32]>,
    previous: Option<([u8; 32], DateTime<Utc>)>,
}

pub struct ServiceCredentials {
    services: [ServiceCredential; 2],
}

impl ServiceCredentials {
    pub fn load(path: &Path) -> anyhow::Result<Self> {
        Self::parse(&read_file(path, true)?, Utc::now())
    }

    pub(super) fn parse(bytes: &[u8], now: DateTime<Utc>) -> anyhow::Result<Self> {
        let document: CredentialDocument = serde_json::from_slice(bytes)
            .map_err(|_| anyhow::anyhow!("Invalid internal credential document"))?;
        let mut unique_digests = HashSet::new();

        Ok(Self {
            services: [
                parse_service(Identity::Core, document.core, now, &mut unique_digests)?,
                parse_service(Identity::Files, document.files, now, &mut unique_digests)?,
            ],
        })
    }

    /// Returns an authenticated identity and its single permission, independently.
    pub fn authenticate(
        &self,
        headers: &HeaderMap,
        now: DateTime<Utc>,
    ) -> Option<(Identity, bool)> {
        let token = bearer_token(headers)?;
        let digest = token_digest(token)?;
        let mut identity = None;
        // Check every slot and both generations without stopping at the first match.
        for service in &self.services {
            let matched = service.matches_digest(&digest, now);
            if matched && service.enabled {
                identity = Some((service.identity, service.allowed));
            }
        }

        identity
    }
}

fn parse_service(
    identity: Identity,
    entry: CredentialEntry,
    now: DateTime<Utc>,
    unique_digests: &mut HashSet<[u8; 32]>,
) -> anyhow::Result<ServiceCredential> {
    ensure!(
        entry.permissions.len() <= 1
            && entry
                .permissions
                .iter()
                .all(|permission| permission == "sessions:check"),
        "Invalid internal permissions"
    );
    ensure!(
        !entry.enabled || entry.active_sha256.is_some(),
        "Enabled service requires active credential"
    );

    let active = entry
        .active_sha256
        .as_deref()
        .map(|value| parse_unique_digest(value, unique_digests))
        .transpose()?;
    let previous = entry
        .previous
        .map(|previous| -> anyhow::Result<_> {
            let expires = DateTime::parse_from_rfc3339(&previous.expires_at)
                .map_err(|_| anyhow::anyhow!("Invalid previous credential expiration"))?
                .with_timezone(&Utc);
            ensure!(
                expires <= now + std::time::Duration::from_secs(900),
                "PreviousCredential credential overlap exceeds 15 minutes"
            );
            ensure!(
                active.is_some(),
                "PreviousCredential credential requires active credential"
            );
            Ok((
                parse_unique_digest(&previous.sha256, unique_digests)?,
                expires,
            ))
        })
        .transpose()?;

    Ok(ServiceCredential {
        identity,
        enabled: entry.enabled,
        allowed: !entry.permissions.is_empty(),
        active,
        previous,
    })
}

fn parse_unique_digest(
    value: &str,
    unique_digests: &mut HashSet<[u8; 32]>,
) -> anyhow::Result<[u8; 32]> {
    let digest = decode_digest(value)
        .ok_or_else(|| anyhow::anyhow!("Invalid internal credential digest"))?;
    ensure!(
        unique_digests.insert(digest),
        "Duplicate internal credential digest"
    );
    Ok(digest)
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let mut authorization_values = headers.get_all(AUTHORIZATION).iter();
    let authorization_header = authorization_values.next()?.to_str().ok()?;
    if authorization_values.next().is_some() {
        return None;
    }
    let (scheme, token) = authorization_header.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("Bearer") {
        return None;
    }
    Some(token)
}

impl ServiceCredential {
    fn matches_digest(&self, digest: &[u8; 32], now: DateTime<Utc>) -> bool {
        let active_match = self.active.unwrap_or([0; 32]).ct_eq(digest);
        let (previous_digest, expires_at) =
            self.previous.unwrap_or(([0; 32], DateTime::UNIX_EPOCH));
        let previous_match = previous_digest.ct_eq(digest);

        bool::from(active_match) && self.active.is_some()
            || bool::from(previous_match) && self.previous.is_some() && now < expires_at
    }
}

fn decode_digest(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return None;
    }
    let mut bytes = [0; 32];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&value[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}

fn token_digest(token: &str) -> Option<[u8; 32]> {
    let encoded = token.strip_prefix("svc_")?;
    if encoded.len() != 43 {
        return None;
    }
    let bytes = Zeroizing::new(URL_SAFE_NO_PAD.decode(encoded).ok()?);
    if bytes.len() != 32 || URL_SAFE_NO_PAD.encode(&bytes) != encoded {
        return None;
    }
    Some(Sha256::digest(token.as_bytes()).into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::HeaderValue;

    fn secret(seed: u8) -> String {
        format!("svc_{}", URL_SAFE_NO_PAD.encode([seed; 32]))
    }
    fn digest(seed: u8) -> String {
        Sha256::digest(secret(seed).as_bytes())
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }
    fn document() -> serde_json::Value {
        serde_json::json!({
            "core":{"enabled":true,"permissions":["sessions:check"],"active_sha256":digest(1)},
            "files":{"enabled":true,"permissions":[],"active_sha256":digest(2)}
        })
    }
    fn headers(token: &str) -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {token}")).unwrap(),
        );
        headers
    }
    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).unwrap()
    }
    fn parse(doc: serde_json::Value) -> ServiceCredentials {
        ServiceCredentials::parse(&serde_json::to_vec(&doc).unwrap(), now()).unwrap()
    }

    #[test]
    fn authenticates_independent_identities_and_separates_permission() {
        let credentials = parse(document());
        assert_eq!(
            credentials.authenticate(&headers(&secret(1)), now()),
            Some((Identity::Core, true))
        );
        assert_eq!(
            credentials.authenticate(&headers(&secret(2)), now()),
            Some((Identity::Files, false))
        );
        assert_eq!(credentials.authenticate(&headers(&secret(3)), now()), None);
        assert_eq!(credentials.authenticate(&HeaderMap::new(), now()), None);
        assert_eq!(
            credentials.authenticate(&headers("eyJhbGciOiJFZERTQSJ9.payload.signature"), now()),
            None
        );
        let mut duplicated = headers(&secret(1));
        duplicated.append(
            AUTHORIZATION,
            HeaderValue::from_str(&format!("Bearer {}", secret(2))).unwrap(),
        );
        assert_eq!(credentials.authenticate(&duplicated, now()), None);
    }

    #[test]
    fn previous_expires_at_exact_deadline_and_removal_or_disable_revokes() {
        let mut doc = document();
        let deadline = now() + std::time::Duration::from_secs(60);
        doc["core"]["previous"] =
            serde_json::json!({"sha256":digest(3),"expires_at":deadline.to_rfc3339()});
        let credentials = parse(doc.clone());
        assert_eq!(
            credentials.authenticate(
                &headers(&secret(3)),
                deadline - std::time::Duration::from_micros(1)
            ),
            Some((Identity::Core, true))
        );
        assert_eq!(
            credentials.authenticate(&headers(&secret(3)), deadline),
            None
        );
        assert_eq!(
            credentials.authenticate(&headers(&secret(1)), deadline),
            Some((Identity::Core, true))
        );
        // A restart does not recalculate the expiration or revive an expired previous key.
        let restarted =
            ServiceCredentials::parse(&serde_json::to_vec(&doc).unwrap(), deadline).unwrap();
        assert_eq!(restarted.authenticate(&headers(&secret(3)), deadline), None);
        doc["core"].as_object_mut().unwrap().remove("previous");
        assert_eq!(
            parse(doc.clone()).authenticate(&headers(&secret(3)), now()),
            None
        );
        doc["core"]["enabled"] = false.into();
        assert_eq!(parse(doc).authenticate(&headers(&secret(1)), now()), None);
    }

    #[test]
    fn rejects_ambiguous_incomplete_or_overlong_overlap_configuration_without_contents() {
        let original = document();
        let mut invalid = Vec::new();
        let mut doc = original.clone();
        doc["files"]["active_sha256"] = digest(1).into();
        invalid.push(doc);
        let mut doc = original.clone();
        doc["core"]["active_sha256"] = serde_json::Value::Null;
        invalid.push(doc);
        let mut doc = original.clone();
        doc["core"]["permissions"] = serde_json::json!(["admin"]);
        invalid.push(doc);
        let mut doc = original.clone();
        doc["core"]["permissions"] = serde_json::json!(["sessions:check", "sessions:check"]);
        invalid.push(doc);
        let mut doc = original.clone();
        doc["core"]["secret"] = secret(1).into();
        invalid.push(doc);
        let mut doc = original.clone();
        doc["core"]["previous"] = serde_json::json!({"sha256":digest(3),"expires_at":(now()+std::time::Duration::from_secs(901)).to_rfc3339()});
        invalid.push(doc);
        let mut doc = original;
        doc["core"]["active_sha256"] = secret(1).into();
        invalid.push(doc);
        for doc in invalid {
            let error = ServiceCredentials::parse(&serde_json::to_vec(&doc).unwrap(), now())
                .err()
                .expect("must reject");
            assert!(!format!("{error:?}").contains(&secret(1)));
        }
        let duplicate_fields = br#"{"core":{},"core":{},"files":{}}"#;
        assert!(ServiceCredentials::parse(duplicate_fields, now()).is_err());
    }
}
