use std::{collections::HashSet, path::PathBuf};

use anyhow::{ensure, Context};

#[derive(Clone)]
pub struct TokenConfig {
    pub issuer: String,
    pub audiences: Vec<String>,
    pub active_kid: String,
    pub private_key_path: PathBuf,
    pub jwks_path: PathBuf,
}

impl TokenConfig {
    pub fn from_env() -> anyhow::Result<Self> {
        let required =
            |name| std::env::var(name).with_context(|| format!("Missing or invalid {name}"));
        let config = Self {
            issuer: required("AUTH_JWT_ISSUER")?,
            audiences: required("AUTH_JWT_AUDIENCES")?
                .split(',')
                .map(str::to_owned)
                .collect(),
            active_kid: required("AUTH_JWT_ACTIVE_KID")?,
            private_key_path: required("AUTH_JWT_PRIVATE_KEY_PATH")?.into(),
            jwks_path: required("AUTH_JWT_JWKS_PATH")?.into(),
        };

        config.validate()?;

        Ok(config)
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        let issuer = reqwest::Url::parse(&self.issuer)
            .map_err(|_| anyhow::anyhow!("AUTH_JWT_ISSUER must be an absolute HTTPS URL"))?;

        ensure!(
            issuer.scheme() == "https"
                && issuer.host_str().is_some()
                && issuer.username().is_empty()
                && issuer.password().is_none()
                && issuer.query().is_none()
                && issuer.fragment().is_none(),
            "AUTH_JWT_ISSUER must be HTTPS without credentials, query or fragment"
        );

        let mut seen = HashSet::new();

        ensure!(
            (1..=16).contains(&self.audiences.len())
                && self.audiences.iter().all(|aud| !aud.is_empty()
                    && aud.len() <= 128
                    && aud.bytes().all(|b| b.is_ascii_graphic())
                    && seen.insert(aud)),
            "AUTH_JWT_AUDIENCES must contain distinct nonempty identifiers without whitespace"
        );
        ensure!(
            !self.private_key_path.as_os_str().is_empty() && !self.jwks_path.as_os_str().is_empty(),
            "Key file paths must not be empty"
        );
        ensure!(
            !self.active_kid.is_empty(),
            "AUTH_JWT_ACTIVE_KID must not be empty"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_untrusted_or_ambiguous_configuration() {
        let mut config = TokenConfig {
            issuer: "https://auth.example.invalid".into(),
            audiences: vec!["sonimbus-core".into(), "sonimbus-files".into()],
            active_kid: "test-only".into(),
            private_key_path: "test-only.pem".into(),
            jwks_path: "public.json".into(),
        };

        assert!(config.validate().is_ok());

        for issuer in [
            "",
            "http://auth.example.invalid",
            "https://user:pass@auth.example.invalid",
            "https://auth.example.invalid?key=value",
            "https://auth.example.invalid#x",
        ] {
            config.issuer = issuer.into();
            assert!(config.validate().is_err());
        }
        config.issuer = "https://auth.example.invalid".into();

        for audiences in [vec![], vec![""], vec!["core", "core"], vec![" core"]] {
            config.audiences = audiences.into_iter().map(str::to_owned).collect();
            assert!(config.validate().is_err());
        }
    }
}
