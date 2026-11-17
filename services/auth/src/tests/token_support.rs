//! Disposable test fixtures only. No environment secrets and no network access.
use super::support::TemporaryDirectory;
use std::{io::Write, sync::Arc};

use auth_service::tokens::{config::TokenConfig, issuer::AccessTokenIssuer};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{pkcs8::EncodePrivateKey, SigningKey};

pub fn issuer() -> Arc<AccessTokenIssuer> {
    let directory = TemporaryDirectory::new("sonimbus-issuer-test-");
    let private_key_path = directory.path().join("test-only.pem");
    let jwks_path = directory.path().join("public.json");
    let signing_key = SigningKey::from_bytes(&[7; 32]);
    let mut open_options = std::fs::OpenOptions::new();
    open_options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        open_options.mode(0o600);
    }
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
    let public_keys = serde_json::json!({
        "keys": [{
            "kty": "OKP",
            "crv": "Ed25519",
            "use": "sig",
            "alg": "EdDSA",
            "kid": "test-only",
            "x": URL_SAFE_NO_PAD.encode(signing_key.verifying_key().as_bytes()),
        }],
    });
    std::fs::write(&jwks_path, serde_json::to_vec(&public_keys).unwrap()).unwrap();
    let issuer = AccessTokenIssuer::load(TokenConfig {
        issuer: "https://auth.example.invalid".into(),
        audiences: vec![
            "sonimbus-core".into(),
            "sonimbus-files".into(),
            "sonimbus-auth".into(),
        ],
        active_kid: "test-only".into(),
        private_key_path,
        jwks_path,
    })
    .unwrap();
    Arc::new(issuer)
}
