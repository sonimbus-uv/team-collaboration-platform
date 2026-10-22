use std::{collections::HashSet, fs::File, io::Read, path::Path};

use anyhow::{ensure, Context};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{pkcs8::DecodePrivateKey, SigningKey, VerifyingKey};
use serde::{Deserialize, Serialize};
use zeroize::Zeroizing;

/// Public-only representation. Unknown fields (including private `d`) are rejected.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PublicJwk {
    pub kty: String,
    pub crv: String,
    pub r#use: String,
    pub alg: String,
    pub kid: String,
    pub x: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Jwks {
    pub keys: Vec<PublicJwk>,
}

/// Immutable signing/public snapshot: reloads require a validated process restart.
/// Deliberately has no Debug/Serialize implementation.
pub struct KeySet {
    signing: SigningKey,
    active_kid: String,
    public: Jwks,
}

impl KeySet {
    pub fn load(private_path: &Path, active_kid: &str, jwks_path: &Path) -> anyhow::Result<Self> {
        let private = Zeroizing::new(read_file(private_path, true)?);
        let pem = std::str::from_utf8(&private)
            .map_err(|_| anyhow::anyhow!("Signing key must be PKCS#8 PEM"))?;
        let signing = SigningKey::from_pkcs8_pem(pem)
            .map_err(|_| anyhow::anyhow!("Signing key must be unencrypted Ed25519 PKCS#8 PEM"))?;
        let public: Jwks = serde_json::from_slice(&read_file(jwks_path, false)?)
            .map_err(|_| anyhow::anyhow!("Invalid public JWKS document"))?;
        Self::new(signing, active_kid, public)
    }

    pub(super) fn new(
        signing: SigningKey,
        active_kid: &str,
        mut public: Jwks,
    ) -> anyhow::Result<Self> {
        ensure!(valid_kid(active_kid), "Invalid active signing kid");
        ensure!(
            (1..=16).contains(&public.keys.len()),
            "JWKS must contain 1 to 16 keys"
        );
        let mut kids = HashSet::new();
        let mut active_matches = false;
        for key in &public.keys {
            ensure!(
                valid_kid(&key.kid) && kids.insert(&key.kid),
                "Invalid or duplicate JWKS kid"
            );
            ensure!(
                key.kty == "OKP"
                    && key.crv == "Ed25519"
                    && key.r#use == "sig"
                    && key.alg == "EdDSA",
                "JWKS keys must be Ed25519 signature keys"
            );
            let decoded = URL_SAFE_NO_PAD
                .decode(&key.x)
                .map_err(|_| anyhow::anyhow!("Invalid JWK public key encoding"))?;
            let bytes: [u8; 32] = decoded
                .try_into()
                .map_err(|_| anyhow::anyhow!("Ed25519 public key must contain 32 bytes"))?;
            ensure!(
                URL_SAFE_NO_PAD.encode(bytes) == key.x,
                "Noncanonical JWK public key"
            );
            let verifying = VerifyingKey::from_bytes(&bytes)
                .map_err(|_| anyhow::anyhow!("Invalid Ed25519 public key"))?;
            ensure!(!verifying.is_weak(), "Weak Ed25519 public key");
            if key.kid == active_kid {
                ensure!(
                    verifying == signing.verifying_key(),
                    "Active private key and public JWK differ"
                );
                active_matches = true;
            }
        }
        ensure!(active_matches, "Active signing kid is missing from JWKS");
        public.keys.sort_by(|a, b| a.kid.cmp(&b.kid));
        Ok(Self {
            signing,
            active_kid: active_kid.into(),
            public,
        })
    }

    pub fn public(&self) -> &Jwks {
        &self.public
    }
    pub fn active_kid(&self) -> &str {
        &self.active_kid
    }
    pub(super) fn signing(&self) -> &SigningKey {
        &self.signing
    }

    #[cfg(test)]
    pub(crate) fn for_tests() -> Self {
        // Disposable fixture derived from fixed bytes; never deployment material.
        let signing = SigningKey::from_bytes(&[7; 32]);
        let public = Jwks {
            keys: vec![test_jwk("test-only", &signing)],
        };
        Self::new(signing, "test-only", public).unwrap()
    }
}

fn valid_kid(kid: &str) -> bool {
    (1..=128).contains(&kid.len())
        && kid
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b"._-".contains(&b))
}

fn read_file(path: &Path, private: bool) -> anyhow::Result<Vec<u8>> {
    // No parser errors, paths or contents are propagated into startup diagnostics.
    ensure!(
        !std::fs::symlink_metadata(path)
            .context("Cannot inspect key file")?
            .file_type()
            .is_symlink(),
        "Key file must not be a symlink"
    );
    let mut options = std::fs::OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file: File = options.open(path).context("Cannot open key file")?;
    let metadata = file.metadata().context("Cannot inspect open key file")?;
    ensure!(metadata.is_file(), "Key path must be a regular file");
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: geteuid has no arguments or memory preconditions.
        ensure!(
            metadata.uid() == unsafe { libc::geteuid() } && metadata.mode() & 0o077 == 0,
            "Private key must be owned by Auth's user and inaccessible to group/others"
        );
    }
    #[cfg(not(unix))]
    let _ = private; // Windows deployments must enforce the documented ACL externally.
    let mut bytes = Vec::new();
    file.take(65_537)
        .read_to_end(&mut bytes)
        .context("Cannot read key file")?;
    ensure!(bytes.len() <= 65_536, "Key file exceeds 64 KiB");
    Ok(bytes)
}

#[cfg(test)]
pub(super) fn test_jwk(kid: &str, signing: &SigningKey) -> PublicJwk {
    PublicJwk {
        kty: "OKP".into(),
        crv: "Ed25519".into(),
        r#use: "sig".into(),
        alg: "EdDSA".into(),
        kid: kid.into(),
        x: URL_SAFE_NO_PAD.encode(signing.verifying_key().as_bytes()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_duplicate_missing_mismatched_and_incompatible_keys() {
        let key = SigningKey::from_bytes(&[7; 32]);
        let valid = test_jwk("active", &key);
        assert!(KeySet::new(key.clone(), "active", Jwks { keys: vec![] }).is_err());
        assert!(KeySet::new(
            key.clone(),
            "missing",
            Jwks {
                keys: vec![valid.clone()]
            }
        )
        .is_err());
        assert!(KeySet::new(
            key.clone(),
            "active",
            Jwks {
                keys: vec![valid.clone(), valid.clone()]
            }
        )
        .is_err());
        assert!(KeySet::new(
            SigningKey::from_bytes(&[8; 32]),
            "active",
            Jwks {
                keys: vec![valid.clone()]
            }
        )
        .is_err());
        for field in ["alg", "crv", "kty", "use", "x", "kid"] {
            let mut value = serde_json::to_value(&valid).unwrap();
            value[field] = "invalid!".into();
            let bad = serde_json::from_value(value).unwrap();
            assert!(KeySet::new(key.clone(), "active", Jwks { keys: vec![bad] }).is_err());
        }
        let mut value = serde_json::to_value(&valid).unwrap();
        value["d"] = "private-not-allowed".into();
        assert!(serde_json::from_value::<PublicJwk>(value).is_err());
    }

    #[test]
    fn rotation_keeps_public_keys_and_changes_active_selection() {
        let old = SigningKey::from_bytes(&[7; 32]);
        let new = SigningKey::from_bytes(&[8; 32]);
        let public = Jwks {
            keys: vec![test_jwk("old", &old), test_jwk("new", &new)],
        };
        let first = KeySet::new(old, "old", public.clone()).unwrap();
        let next = KeySet::new(new, "new", public).unwrap();
        assert_eq!(
            serde_json::to_value(first.public()).unwrap(),
            serde_json::to_value(next.public()).unwrap()
        );
        assert_eq!(first.active_kid(), "old");
        assert_eq!(next.active_kid(), "new");
    }

    #[test]
    fn loads_disposable_pkcs8_and_rejects_bad_files() {
        use ed25519_dalek::pkcs8::EncodePrivateKey;
        let dir = std::env::temp_dir().join(format!("sonimbus-jwks-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&dir).unwrap();
        let private = dir.join("test-only.pem");
        let public = dir.join("public.json");
        let key = SigningKey::from_bytes(&[7; 32]);
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        use std::io::Write;
        options
            .open(&private)
            .unwrap()
            .write_all(key.to_pkcs8_pem(Default::default()).unwrap().as_bytes())
            .unwrap();
        std::fs::write(
            &public,
            serde_json::to_vec(KeySet::for_tests().public()).unwrap(),
        )
        .unwrap();
        assert!(KeySet::load(&private, "test-only", &public).is_ok());
        assert!(KeySet::load(&dir.join("missing"), "test-only", &public).is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o644)).unwrap();
            assert!(KeySet::load(&private, "test-only", &public).is_err());
            std::fs::set_permissions(&private, std::fs::Permissions::from_mode(0o600)).unwrap();
            let link = dir.join("symlink");
            std::os::unix::fs::symlink(&private, &link).unwrap();
            assert!(KeySet::load(&link, "test-only", &public).is_err());
            std::fs::remove_file(link).unwrap();
        }
        std::fs::write(&private, "invalid-test-material").unwrap();
        assert!(KeySet::load(&private, "test-only", &public).is_err());
        // Only these test-created files are removed; never user paths.
        std::fs::remove_file(private).unwrap();
        std::fs::remove_file(public).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
