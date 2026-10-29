//! Disposable test fixtures only. No environment secrets and no network access.
use std::{io::Write, sync::Arc};

use auth_service::tokens::{config::TokenConfig, issuer::AccessTokenIssuer};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use ed25519_dalek::{pkcs8::EncodePrivateKey, SigningKey};

pub fn issuer() -> Arc<AccessTokenIssuer> {
    let directory =
        std::env::temp_dir().join(format!("sonimbus-issuer-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&directory).unwrap();
    let private = directory.join("test-only.pem");
    let public = directory.join("public.json");
    let key = SigningKey::from_bytes(&[7; 32]);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    options
        .open(&private)
        .unwrap()
        .write_all(key.to_pkcs8_pem(Default::default()).unwrap().as_bytes())
        .unwrap();
    let jwks = serde_json::json!({"keys":[{"kty":"OKP","crv":"Ed25519","use":"sig","alg":"EdDSA","kid":"test-only","x":URL_SAFE_NO_PAD.encode(key.verifying_key().as_bytes())}]});
    std::fs::write(&public, serde_json::to_vec(&jwks).unwrap()).unwrap();
    let issuer = AccessTokenIssuer::load(TokenConfig {
        issuer: "https://auth.example.invalid".into(),
        audiences: vec![
            "sonimbus-core".into(),
            "sonimbus-files".into(),
            "sonimbus-auth".into(),
        ],
        active_kid: "test-only".into(),
        private_key_path: private.clone(),
        jwks_path: public.clone(),
    })
    .unwrap();
    std::fs::remove_file(private).unwrap();
    std::fs::remove_file(public).unwrap();
    std::fs::remove_dir(directory).unwrap();
    Arc::new(issuer)
}
