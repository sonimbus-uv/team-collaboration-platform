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

/// Immutable key snapshot; restart to reload. No Debug/Serialize to protect signing material.
pub struct KeySet {
    signing: SigningKey,
    active_kid: String,
    public: Jwks,
}

impl KeySet {
    pub fn load(
        private_key_path: &Path,
        active_kid: &str,
        jwks_path: &Path,
    ) -> anyhow::Result<Self> {
        let private_key_bytes = Zeroizing::new(read_file(private_key_path, true)?);
        let private_key_pem = std::str::from_utf8(&private_key_bytes)
            .map_err(|_| anyhow::anyhow!("Signing key must be PKCS#8 PEM"))?;
        let signing_key = SigningKey::from_pkcs8_pem(private_key_pem)
            .map_err(|_| anyhow::anyhow!("Signing key must be unencrypted Ed25519 PKCS#8 PEM"))?;
        let public_keys: Jwks = serde_json::from_slice(&read_file(jwks_path, false)?)
            .map_err(|_| anyhow::anyhow!("Invalid public JWKS document"))?;

        Self::new(signing_key, active_kid, public_keys)
    }

    pub(super) fn new(
        signing_key: SigningKey,
        active_kid: &str,
        mut public_keys: Jwks,
    ) -> anyhow::Result<Self> {
        ensure!(valid_kid(active_kid), "Invalid active signing kid");

        ensure!(
            (1..=16).contains(&public_keys.keys.len()),
            "JWKS must contain 1 to 16 keys"
        );

        let mut unique_key_ids = HashSet::new();
        let mut active_key_found = false;

        for key in &public_keys.keys {
            ensure!(
                valid_kid(&key.kid) && unique_key_ids.insert(&key.kid),
                "Invalid or duplicate JWKS kid"
            );
            ensure!(
                key.kty == "OKP"
                    && key.crv == "Ed25519"
                    && key.r#use == "sig"
                    && key.alg == "EdDSA",
                "JWKS keys must be Ed25519 signature keys"
            );

            let decoded_public_key = URL_SAFE_NO_PAD
                .decode(&key.x)
                .map_err(|_| anyhow::anyhow!("Invalid JWK public key encoding"))?;
            let public_key_bytes: [u8; 32] = decoded_public_key
                .try_into()
                .map_err(|_| anyhow::anyhow!("Ed25519 public key must contain 32 bytes"))?;

            ensure!(
                URL_SAFE_NO_PAD.encode(public_key_bytes) == key.x,
                "Noncanonical JWK public key"
            );

            let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
                .map_err(|_| anyhow::anyhow!("Invalid Ed25519 public key"))?;

            ensure!(!verifying_key.is_weak(), "Weak Ed25519 public key");

            if key.kid == active_kid {
                ensure!(
                    verifying_key == signing_key.verifying_key(),
                    "Active private key and public JWK differ"
                );

                active_key_found = true;
            }
        }

        ensure!(active_key_found, "Active signing kid is missing from JWKS");

        public_keys
            .keys
            .sort_by(|left_key, right_key| left_key.kid.cmp(&right_key.kid));

        Ok(Self {
            signing: signing_key,
            active_kid: active_kid.into(),
            public: public_keys,
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
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte))
}

fn read_file(path: &Path, is_private_key: bool) -> anyhow::Result<Vec<u8>> {
    ensure!(
        !std::fs::symlink_metadata(path)
            .context("Cannot inspect key file")?
            .file_type()
            .is_symlink(),
        "Key file must not be a symlink"
    );

    let mut open_options = std::fs::OpenOptions::new();
    open_options.read(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open_options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }

    let file: File = open_options.open(path).context("Cannot open key file")?;
    let file_metadata = file.metadata().context("Cannot inspect open key file")?;

    ensure!(file_metadata.is_file(), "Key path must be a regular file");

    #[cfg(unix)]
    if is_private_key {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: geteuid has no arguments or memory preconditions.
        ensure!(
            file_metadata.uid() == unsafe { libc::geteuid() } && file_metadata.mode() & 0o077 == 0,
            "Private key must be owned by Auth's user and inaccessible to group/others"
        );
    }

    #[cfg(not(unix))]
    let _ = is_private_key; // Windows deployments must enforce the documented ACL externally.

    let mut file_contents = Vec::new();

    file.take(65_537)
        .read_to_end(&mut file_contents)
        .context("Cannot read key file")?;

    ensure!(file_contents.len() <= 65_536, "Key file exceeds 64 KiB");

    Ok(file_contents)
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
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let valid_public_key = test_jwk("active", &signing_key);
        assert!(KeySet::new(signing_key.clone(), "active", Jwks { keys: vec![] }).is_err());
        assert!(KeySet::new(
            signing_key.clone(),
            "missing",
            Jwks {
                keys: vec![valid_public_key.clone()]
            }
        )
        .is_err());
        assert!(KeySet::new(
            signing_key.clone(),
            "active",
            Jwks {
                keys: vec![valid_public_key.clone(), valid_public_key.clone()]
            }
        )
        .is_err());
        assert!(KeySet::new(
            SigningKey::from_bytes(&[8; 32]),
            "active",
            Jwks {
                keys: vec![valid_public_key.clone()]
            }
        )
        .is_err());

        for field in ["alg", "crv", "kty", "use", "x", "kid"] {
            let mut jwk_document = serde_json::to_value(&valid_public_key).unwrap();
            jwk_document[field] = "invalid!".into();
            let invalid_public_key = serde_json::from_value(jwk_document).unwrap();
            assert!(KeySet::new(
                signing_key.clone(),
                "active",
                Jwks {
                    keys: vec![invalid_public_key]
                }
            )
            .is_err());
        }

        let mut jwk_document = serde_json::to_value(&valid_public_key).unwrap();
        jwk_document["d"] = "private-not-allowed".into();
        assert!(serde_json::from_value::<PublicJwk>(jwk_document).is_err());
    }

    #[test]
    fn rotation_keeps_public_keys_and_changes_active_selection() {
        let previous_signing_key = SigningKey::from_bytes(&[7; 32]);
        let current_signing_key = SigningKey::from_bytes(&[8; 32]);
        let public_keys = Jwks {
            keys: vec![
                test_jwk("old", &previous_signing_key),
                test_jwk("new", &current_signing_key),
            ],
        };
        let previous_key_set =
            KeySet::new(previous_signing_key, "old", public_keys.clone()).unwrap();
        let current_key_set = KeySet::new(current_signing_key, "new", public_keys).unwrap();
        assert_eq!(
            serde_json::to_value(previous_key_set.public()).unwrap(),
            serde_json::to_value(current_key_set.public()).unwrap()
        );
        assert_eq!(previous_key_set.active_kid(), "old");
        assert_eq!(current_key_set.active_kid(), "new");
    }

    #[test]
    fn loads_disposable_pkcs8_and_rejects_bad_files() {
        use ed25519_dalek::pkcs8::EncodePrivateKey;

        let test_directory =
            std::env::temp_dir().join(format!("sonimbus-jwks-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&test_directory).unwrap();
        let private_key_path = test_directory.join("test-only.pem");
        let jwks_path = test_directory.join("public.json");
        let signing_key = SigningKey::from_bytes(&[7; 32]);
        let mut open_options = std::fs::OpenOptions::new();

        open_options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            open_options.mode(0o600);
        }
        use std::io::Write;
        open_options
            .open(&private_key_path)
            .unwrap()
            .write_all(
                signing_key
                    .to_pkcs8_pem(Default::default())
                    .unwrap()
                    .as_bytes(),
            )
            .unwrap();
        std::fs::write(
            &jwks_path,
            serde_json::to_vec(KeySet::for_tests().public()).unwrap(),
        )
        .unwrap();
        assert!(KeySet::load(&private_key_path, "test-only", &jwks_path).is_ok());
        assert!(KeySet::load(&test_directory.join("missing"), "test-only", &jwks_path).is_err());
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&private_key_path, std::fs::Permissions::from_mode(0o644))
                .unwrap();
            assert!(KeySet::load(&private_key_path, "test-only", &jwks_path).is_err());
            std::fs::set_permissions(&private_key_path, std::fs::Permissions::from_mode(0o600))
                .unwrap();
            let symlink_path = test_directory.join("symlink");
            std::os::unix::fs::symlink(&private_key_path, &symlink_path).unwrap();
            assert!(KeySet::load(&symlink_path, "test-only", &jwks_path).is_err());
            std::fs::remove_file(symlink_path).unwrap();
        }
        std::fs::write(&private_key_path, "invalid-test-material").unwrap();
        assert!(KeySet::load(&private_key_path, "test-only", &jwks_path).is_err());
        std::fs::remove_file(private_key_path).unwrap();
        std::fs::remove_file(jwks_path).unwrap();
        std::fs::remove_dir(test_directory).unwrap();
    }
}
