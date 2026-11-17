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

    pub fn verify_for_auth(&self, token: &str) -> anyhow::Result<AccessClaims> {
        use jsonwebtoken::{decode, decode_header, DecodingKey, Validation};

        anyhow::ensure!(token.len() <= 8192, "Invalid access token");

        let header = decode_header(token).map_err(|_| anyhow::anyhow!("Invalid access token"))?;
        anyhow::ensure!(
            header.alg == Algorithm::EdDSA && header.typ.as_deref() == Some("at+jwt"),
            "Invalid access token"
        );

        let key = self
            .public_keys()
            .keys
            .iter()
            .find(|k| Some(k.kid.as_str()) == header.kid.as_deref())
            .ok_or_else(|| anyhow::anyhow!("Invalid access token"))?;

        let mut validation = Validation::new(Algorithm::EdDSA);

        validation.leeway = 0;
        validation.set_issuer(&[self.issuer.as_str()]);
        validation.set_audience(&["sonimbus-auth"]);
        validation.set_required_spec_claims(&["exp", "iss", "aud", "sub", "iat"]);

        let claims = decode::<AccessClaims>(
            token,
            &DecodingKey::from_ed_components(&key.x)?,
            &validation,
        )
        .map_err(|_| anyhow::anyhow!("Invalid access token"))?
        .claims;

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
        anyhow::ensure!(
            !claims.sub.is_nil()
                && !claims.sid.is_nil()
                && !claims.jti.is_nil()
                && claims.iat <= now
                && claims.exp > now
                && claims.exp > claims.iat
                && claims.exp - claims.iat <= 900,
            "Invalid access token"
        );

        Ok(claims)
    }

    pub fn public_keys(&self) -> &Jwks {
        self.keys.public()
    }

    pub fn issue(&self, session: &SessionContext) -> anyhow::Result<String> {
        self.issue_with_lifetime(session).map(|(token, _)| token)
    }

    pub fn issue_with_lifetime(&self, session: &SessionContext) -> anyhow::Result<(String, u64)> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|_| anyhow::anyhow!("System clock is before Unix epoch"))?
            .as_secs();
        let exp = session.access_expiration(now)?;

        Ok((self.issue_at(session, now)?, exp - now))
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

    #[test]
    fn auth_verification_requires_signature_type_known_key_audience_and_expiry() {
        let issuer = issuer();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let token = issuer.issue_at(&session(now, 900), now).unwrap();

        assert!(issuer.verify_for_auth(&token).is_ok());

        let claims = issuer.verify_for_auth(&token).unwrap();
        let mut header = Header::new(Algorithm::EdDSA);
        header.typ = Some("at+jwt".into());
        header.kid = Some(issuer.keys.active_kid().into());

        for typ in ["JWT", "svc"] {
            header.typ = Some(typ.into());
            let token = encode(&header, &claims, &issuer.encoding).unwrap();
            assert!(issuer.verify_for_auth(&token).is_err());
        }

        header.typ = Some("at+jwt".into());
        let mut wrong = claims.clone();
        wrong.aud = vec!["sonimbus-core".into()];
        assert!(issuer
            .verify_for_auth(&encode(&header, &wrong, &issuer.encoding).unwrap())
            .is_err());

        wrong = claims.clone();
        wrong.exp = now - 1;
        assert!(issuer
            .verify_for_auth(&encode(&header, &wrong, &issuer.encoding).unwrap())
            .is_err());

        header.kid = Some("unknown".into());
        assert!(issuer
            .verify_for_auth(&encode(&header, &claims, &issuer.encoding).unwrap())
            .is_err());

        assert!(issuer.verify_for_auth("svc_invalid").is_err());
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
    #[test]
    fn auth_verification_rejects_malformed_and_oversized_tokens() {
        let issuer = issuer();
        for token in ["", "not-a-jwt", "a.b", "a.b.c.d", "@@@.@@@.@@@"] {
            assert!(issuer.verify_for_auth(token).is_err());
        }
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let valid_token = issuer.issue_at(&session(now, 900), now).unwrap();
        let mut claims = issuer.verify_for_auth(&valid_token).unwrap();
        claims.aud.push("a".repeat(8192));
        let mut header = Header::new(Algorithm::EdDSA);
        header.typ = Some("at+jwt".into());
        header.kid = Some(issuer.keys.active_kid().into());
        let oversized_token = encode(&header, &claims, &issuer.encoding).unwrap();
        assert!(oversized_token.len() > 8192);
        assert!(issuer.verify_for_auth(&oversized_token).is_err());
    }

    #[test]
    fn auth_verification_rejects_missing_headers_and_wrong_algorithms() {
        let issuer = issuer();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let valid_token = issuer.issue_at(&session(now, 900), now).unwrap();
        let claims = issuer.verify_for_auth(&valid_token).unwrap();
        for (token_type, key_id) in [
            (None, Some(issuer.keys.active_kid().to_owned())),
            (Some("at+jwt".to_owned()), None),
        ] {
            let mut header = Header::new(Algorithm::EdDSA);
            header.typ = token_type;
            header.kid = key_id;
            let token = encode(&header, &claims, &issuer.encoding).unwrap();
            assert!(issuer.verify_for_auth(&token).is_err());
        }
        let mut header = Header::new(Algorithm::HS256);
        header.typ = Some("at+jwt".into());
        header.kid = Some(issuer.keys.active_kid().into());
        let token = encode(
            &header,
            &claims,
            &EncodingKey::from_secret(b"test-only-hmac-key"),
        )
        .unwrap();
        assert!(issuer.verify_for_auth(&token).is_err());
    }

    #[test]
    fn auth_verification_rejects_nil_identifiers_and_invalid_time_claims() {
        let issuer = issuer();
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let valid_token = issuer.issue_at(&session(now, 900), now).unwrap();
        let claims = issuer.verify_for_auth(&valid_token).unwrap();
        let mut header = Header::new(Algorithm::EdDSA);
        header.typ = Some("at+jwt".into());
        header.kid = Some(issuer.keys.active_kid().into());
        let mut invalid_claims = Vec::new();
        for identifier in ["sub", "sid", "jti"] {
            let mut invalid = claims.clone();
            match identifier {
                "sub" => invalid.sub = Uuid::nil(),
                "sid" => invalid.sid = Uuid::nil(),
                _ => invalid.jti = Uuid::nil(),
            }
            invalid_claims.push(invalid);
        }
        let mut future_issued = claims.clone();
        future_issued.iat = now + 3600;
        future_issued.exp = future_issued.iat + 900;
        invalid_claims.push(future_issued);
        let mut expired = claims.clone();
        expired.exp = now;
        invalid_claims.push(expired);
        let mut reversed_lifetime = claims.clone();
        reversed_lifetime.iat = now;
        reversed_lifetime.exp = now - 1;
        invalid_claims.push(reversed_lifetime);
        let mut excessive_lifetime = claims.clone();
        excessive_lifetime.exp = excessive_lifetime.iat + 901;
        invalid_claims.push(excessive_lifetime);
        for invalid in invalid_claims {
            let token = encode(&header, &invalid, &issuer.encoding).unwrap();
            assert!(issuer.verify_for_auth(&token).is_err());
        }
    }
}
