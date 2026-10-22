use std::time::{SystemTime, UNIX_EPOCH};

use ed25519_dalek::pkcs8::EncodePrivateKey;
use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};
use uuid::Uuid;

use super::{
    claims::{AccessClaims, SessionContext},
    config::TokenConfig,
    keys::{Jwks, KeySet},
};

/// No Debug/Serialize: neither private material nor returned tokens belong in logs.
pub struct AccessTokenIssuer {
    keys: KeySet,
    encoding: EncodingKey,
    issuer: String,
    audiences: Vec<String>,
}

impl AccessTokenIssuer {
    pub fn load(config: TokenConfig) -> anyhow::Result<Self> {
        config.validate()?;
        let keys = KeySet::load(
            &config.private_key_path,
            &config.active_kid,
            &config.jwks_path,
        )?;
        Self::new(config, keys)
    }

    fn new(config: TokenConfig, keys: KeySet) -> anyhow::Result<Self> {
        config.validate()?;
        anyhow::ensure!(
            config.active_kid == keys.active_kid(),
            "Configured active kid does not match key set"
        );
        let der = keys
            .signing()
            .to_pkcs8_der()
            .map_err(|_| anyhow::anyhow!("Cannot prepare Ed25519 signing key"))?;
        let encoding = EncodingKey::from_ed_der(der.as_bytes());
        Ok(Self {
            keys,
            encoding,
            issuer: config.issuer,
            audiences: config.audiences,
        })
    }

    pub fn public_keys(&self) -> &Jwks {
        self.keys.public()
    }

    pub fn issue(&self, session: &SessionContext) -> anyhow::Result<String> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| anyhow::anyhow!("System clock is before Unix epoch"))?
            .as_secs();
        self.issue_at(session, now)
    }

    fn issue_at(&self, session: &SessionContext, now: u64) -> anyhow::Result<String> {
        let exp = session.access_expiration(now)?;
        let claims = AccessClaims {
            iss: self.issuer.clone(),
            aud: self.audiences.clone(),
            sub: session.account_id,
            sid: session.session_id,
            iat: now,
            exp,
            jti: Uuid::new_v4(),
        };
        let mut header = Header::new(Algorithm::EdDSA);
        header.typ = Some("at+jwt".into());
        header.kid = Some(self.keys.active_kid().into());
        encode(&header, &claims, &self.encoding)
            .map_err(|_| anyhow::anyhow!("Cannot sign access token"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};

    fn issuer() -> AccessTokenIssuer {
        AccessTokenIssuer::new(
            TokenConfig {
                issuer: "https://auth.example.invalid".into(),
                audiences: vec![
                    "sonimbus-core".into(),
                    "sonimbus-files".into(),
                    "sonimbus-auth".into(),
                ],
                active_kid: "test-only".into(),
                private_key_path: "unused-test-path".into(),
                jwks_path: "unused-test-path".into(),
            },
            KeySet::for_tests(),
        )
        .unwrap()
    }

    fn session(now: u64, remaining: u64) -> SessionContext {
        SessionContext {
            account_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            login_at: now - 60,
            expires_at: now + remaining,
        }
    }

    fn verify(
        issuer: &AccessTokenIssuer,
        token: &str,
    ) -> jsonwebtoken::errors::Result<jsonwebtoken::TokenData<AccessClaims>> {
        let jwk = &issuer.public_keys().keys[0];
        let key = DecodingKey::from_ed_components(&jwk.x).unwrap();
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.leeway = 0;
        validation.set_issuer(&["https://auth.example.invalid"]);
        validation.set_audience(&["sonimbus-core"]);
        decode::<AccessClaims>(token, &key, &validation)
    }

    #[test]
    fn emitted_token_verifies_with_published_key_and_expected_claims() {
        let issuer = issuer();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let context = session(now, 1000);
        let token = issuer.issue_at(&context, now).unwrap();
        let data = verify(&issuer, &token).unwrap();
        assert_eq!(data.header.alg, Algorithm::EdDSA);
        assert_eq!(data.header.typ.as_deref(), Some("at+jwt"));
        assert_eq!(
            data.header.kid.as_deref(),
            Some(issuer.public_keys().keys[0].kid.as_str())
        );
        assert_eq!(data.claims.sub, context.account_id);
        assert_eq!(data.claims.sid, context.session_id);
        assert_eq!(data.claims.iat, now);
        assert_eq!(data.claims.exp, now + 900);
        assert_eq!(data.claims.aud, issuer.audiences);
        let next = verify(&issuer, &issuer.issue_at(&context, now).unwrap()).unwrap();
        assert_ne!(next.claims.jti, data.claims.jti);
        assert!(!data.claims.jti.is_nil());
        assert_eq!(decode_header(&token).unwrap().kid, data.header.kid);
    }

    #[test]
    fn caps_access_at_session_expiry_and_rejects_invalid_contexts() {
        let issuer = issuer();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for remaining in [1, 60, 899, 900, 901] {
            let context = session(now, remaining);
            let token = issuer.issue_at(&context, now).unwrap();
            // Decode signed payload directly for boundary assertions without test-clock races.
            use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
            let payload = URL_SAFE_NO_PAD
                .decode(token.split('.').nth(1).unwrap())
                .unwrap();
            let claims: AccessClaims = serde_json::from_slice(&payload).unwrap();
            assert_eq!(claims.exp, now + remaining.min(900));
        }
        let mut context = session(now, 0);
        assert!(issuer.issue_at(&context, now).is_err());
        context.expires_at = now - 1;
        assert!(issuer.issue_at(&context, now).is_err());
        context = session(now, 60);
        context.account_id = Uuid::nil();
        assert!(issuer.issue_at(&context, now).is_err());
        context = session(now, 60);
        context.session_id = Uuid::nil();
        assert!(issuer.issue_at(&context, now).is_err());
        context = session(now, 60);
        context.login_at = now + 1;
        assert!(issuer.issue_at(&context, now).is_err());
        context.login_at = now;
        context.expires_at = now + super::super::claims::MAX_SESSION_SECONDS + 1;
        assert!(issuer.issue_at(&context, now).is_err());
        context.login_at = u64::MAX - 1;
        context.expires_at = u64::MAX;
        assert!(issuer.issue_at(&context, u64::MAX - 1).is_err());
    }

    #[test]
    fn changing_payload_invalidates_signature() {
        use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
        let issuer = issuer();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let token = issuer.issue_at(&session(now, 1000), now).unwrap();
        let mut parts: Vec<_> = token.split('.').map(str::to_owned).collect();
        let mut payload: serde_json::Value =
            serde_json::from_slice(&URL_SAFE_NO_PAD.decode(&parts[1]).unwrap()).unwrap();
        payload["sub"] = Uuid::new_v4().to_string().into();
        parts[1] = URL_SAFE_NO_PAD.encode(serde_json::to_vec(&payload).unwrap());
        assert!(verify(&issuer, &parts.join(".")).is_err());
        let other = DecodingKey::from_ed_der(
            ed25519_dalek::SigningKey::from_bytes(&[8; 32])
                .verifying_key()
                .as_bytes(),
        );
        let mut validation = Validation::new(Algorithm::EdDSA);
        validation.set_audience(&["sonimbus-core"]);
        assert!(decode::<AccessClaims>(&token, &other, &validation).is_err());
    }

    #[test]
    fn both_rotation_generations_verify_against_the_same_public_snapshot() {
        use super::super::keys::test_jwk;
        use ed25519_dalek::SigningKey;
        let old = SigningKey::from_bytes(&[7; 32]);
        let new = SigningKey::from_bytes(&[8; 32]);
        let public = Jwks {
            keys: vec![test_jwk("old", &old), test_jwk("new", &new)],
        };
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        for (kid, signing) in [("old", old), ("new", new)] {
            let config = TokenConfig {
                issuer: "https://auth.example.invalid".into(),
                audiences: vec!["sonimbus-core".into()],
                active_kid: kid.into(),
                private_key_path: "unused-test-path".into(),
                jwks_path: "unused-test-path".into(),
            };
            let issuer =
                AccessTokenIssuer::new(config, KeySet::new(signing, kid, public.clone()).unwrap())
                    .unwrap();
            let token = issuer.issue_at(&session(now, 1000), now).unwrap();
            let header = decode_header(&token).unwrap();
            let key = public
                .keys
                .iter()
                .find(|key| Some(&key.kid) == header.kid.as_ref())
                .unwrap();
            let mut validation = Validation::new(Algorithm::EdDSA);
            validation.set_audience(&["sonimbus-core"]);
            assert!(decode::<AccessClaims>(
                &token,
                &DecodingKey::from_ed_components(&key.x).unwrap(),
                &validation
            )
            .is_ok());
        }
    }
}
