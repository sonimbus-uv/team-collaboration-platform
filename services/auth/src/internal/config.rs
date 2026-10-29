use std::{
    env,
    fs::OpenOptions,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::{ensure, Context};
use ipnet::IpNet;
use zeroize::Zeroizing;

/// Internal API settings are independent of public proxy trust and public quotas.
#[derive(Clone)]
pub struct InternalConfig {
    pub addr: SocketAddr,
    pub allowed_peers: Vec<IpNet>,
    pub credentials_path: PathBuf,
    pub certificate_path: PathBuf,
    pub key_path: PathBuf,
    pub body_limit: usize,
    pub pair_capacity: u32,
    pub pairs_per_second: u32,
    pub concurrent_per_service: u32,
    pub preauth_per_second: u32,
    pub local_limit: usize,
    pub request_timeout: Duration,
    pub sql_timeout: Duration,
    pub redis_connect_timeout: Duration,
    pub redis_operation_timeout: Duration,
    pub lease_ttl: Duration,
    pub handshake_timeout: Duration,
    pub header_timeout: Duration,
}

impl InternalConfig {
    pub fn from_env() -> anyhow::Result<Option<Self>> {
        Self::parse(|name| match env::var(name) {
            Ok(value) => Ok(Some(value)),
            Err(env::VarError::NotPresent) => Ok(None),
            Err(_) => anyhow::bail!("Invalid internal configuration encoding"),
        })
    }

    pub(super) fn parse(
        get: impl Fn(&str) -> anyhow::Result<Option<String>>,
    ) -> anyhow::Result<Option<Self>> {
        match get("AUTH_INTERNAL_ENABLED")?.as_deref() {
            None | Some("false") => return Ok(None),
            Some("true") => {}
            _ => anyhow::bail!("AUTH_INTERNAL_ENABLED must be true or false"),
        }
        let required = |name| -> anyhow::Result<String> {
            let value = get(name)?.context("Missing internal configuration")?;
            ensure!(!value.trim().is_empty(), "Empty internal configuration");
            Ok(value)
        };
        let number = |name, default: u64| -> anyhow::Result<u64> {
            let value = get(name)?
                .map(|s| s.parse::<u64>())
                .transpose()
                .map_err(|_| anyhow::anyhow!("Invalid internal numeric configuration"))?
                .unwrap_or(default);
            ensure!(
                (1..=1_000_000).contains(&value),
                "Internal setting out of range"
            );
            Ok(value)
        };
        let ms = |name, default| number(name, default).map(Duration::from_millis);
        let allowed_peers: Vec<IpNet> = required("AUTH_INTERNAL_ALLOWED_CIDRS")?
            .split(',')
            .map(|s| {
                s.trim()
                    .parse()
                    .map_err(|_| anyhow::anyhow!("Invalid internal CIDR"))
            })
            .collect::<anyhow::Result<_>>()?;
        let config = Self {
            addr: required("AUTH_INTERNAL_ADDR")?
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid internal listen address"))?,
            allowed_peers,
            credentials_path: required("AUTH_INTERNAL_CREDENTIALS_PATH")?.into(),
            certificate_path: required("AUTH_INTERNAL_CERT_PATH")?.into(),
            key_path: required("AUTH_INTERNAL_KEY_PATH")?.into(),
            body_limit: number("AUTH_INTERNAL_BODY_LIMIT_BYTES", 65_536)?.try_into()?,
            pair_capacity: number("AUTH_INTERNAL_PAIR_CAPACITY", 100)?.try_into()?,
            pairs_per_second: number("AUTH_INTERNAL_PAIRS_PER_SECOND", 100)?.try_into()?,
            concurrent_per_service: number("AUTH_INTERNAL_CONCURRENT_PER_SERVICE", 20)?
                .try_into()?,
            preauth_per_second: number("AUTH_INTERNAL_PREAUTH_PER_SECOND", 10)?.try_into()?,
            local_limit: number("AUTH_INTERNAL_LOCAL_LIMIT", 50)?.try_into()?,
            request_timeout: ms("AUTH_INTERNAL_REQUEST_TIMEOUT_MS", 1_000)?,
            sql_timeout: ms("AUTH_INTERNAL_SQL_TIMEOUT_MS", 500)?,
            redis_connect_timeout: ms("AUTH_INTERNAL_REDIS_CONNECT_TIMEOUT_MS", 200)?,
            redis_operation_timeout: ms("AUTH_INTERNAL_REDIS_OPERATION_TIMEOUT_MS", 100)?,
            lease_ttl: ms("AUTH_INTERNAL_LEASE_TTL_MS", 2_000)?,
            handshake_timeout: ms("AUTH_INTERNAL_HANDSHAKE_TIMEOUT_MS", 300)?,
            header_timeout: ms("AUTH_INTERNAL_HEADER_TIMEOUT_MS", 1_000)?,
        };
        config.validate()?;
        Ok(Some(config))
    }

    pub fn validate(&self) -> anyhow::Result<()> {
        ensure!(self.addr.port() != 0, "Internal port must be nonzero");
        ensure!(
            !self.allowed_peers.is_empty()
                && self.allowed_peers.iter().all(|net| net.prefix_len() > 0),
            "Explicit internal source networks are required"
        );
        ensure!(
            self.body_limit > 0 && self.body_limit <= 65_536,
            "Internal body limit exceeds 64 KiB"
        );
        ensure!(
            self.pair_capacity >= 100 && self.pairs_per_second > 0,
            "Internal quota must admit a maximum-size batch"
        );
        ensure!(
            self.concurrent_per_service > 0 && self.local_limit > 0,
            "Invalid internal concurrency"
        );
        ensure!(self.sql_timeout < self.request_timeout
            && self.redis_connect_timeout < self.request_timeout
            && self.redis_operation_timeout < self.request_timeout
            && self.lease_ttl >= self.request_timeout * 2,
            "Internal dependency budgets must fit request timeout and lease must last at least twice that budget");
        Ok(())
    }
}

/// Same bounded, no-follow and Unix owner-only policy used for Auth signing keys.
/// Diagnostics intentionally contain no filenames, parser fragments or contents.
pub(super) fn read_file(path: &Path, private: bool) -> anyhow::Result<Zeroizing<Vec<u8>>> {
    ensure!(
        !std::fs::symlink_metadata(path)
            .map_err(|_| anyhow::anyhow!("Cannot inspect internal file"))?
            .file_type()
            .is_symlink(),
        "Internal files cannot be symlinks"
    );
    let mut options = OpenOptions::new();
    options.read(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
    }
    let file = options
        .open(path)
        .map_err(|_| anyhow::anyhow!("Cannot open internal file"))?;
    let metadata = file
        .metadata()
        .map_err(|_| anyhow::anyhow!("Cannot inspect internal file"))?;
    ensure!(metadata.is_file(), "Internal file must be regular");
    #[cfg(unix)]
    if private {
        use std::os::unix::fs::MetadataExt;
        // SAFETY: geteuid has no memory or argument preconditions.
        ensure!(
            metadata.uid() == unsafe { libc::geteuid() } && metadata.mode() & 0o077 == 0,
            "Internal protected file must be owner-only and owned by Auth"
        );
    }
    #[cfg(not(unix))]
    let _ = private;
    let mut bytes = Zeroizing::new(Vec::new());
    file.take(65_537)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow::anyhow!("Cannot read internal file"))?;
    ensure!(bytes.len() <= 65_536, "Internal file exceeds 64 KiB");
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn values() -> HashMap<&'static str, String> {
        [
            ("AUTH_INTERNAL_ENABLED", "true"),
            ("AUTH_INTERNAL_ADDR", "127.0.0.1:3443"),
            ("AUTH_INTERNAL_ALLOWED_CIDRS", "127.0.0.1/32,::1/128"),
            ("AUTH_INTERNAL_CREDENTIALS_PATH", "test.json"),
            ("AUTH_INTERNAL_CERT_PATH", "test.pem"),
            ("AUTH_INTERNAL_KEY_PATH", "test.key"),
        ]
        .into_iter()
        .map(|(k, v)| (k, v.into()))
        .collect()
    }
    fn parse(values: &HashMap<&str, String>) -> anyhow::Result<Option<InternalConfig>> {
        InternalConfig::parse(|name| Ok(values.get(name).cloned()))
    }

    #[test]
    fn disabled_by_default_and_enabled_configuration_is_explicit() {
        assert!(parse(&HashMap::new()).unwrap().is_none());
        assert!(parse(&values()).unwrap().is_some());
        let mut settings = values();
        settings.remove("AUTH_INTERNAL_KEY_PATH");
        assert!(parse(&settings).is_err());
    }

    #[test]
    fn rejects_unsafe_or_inconsistent_internal_settings() {
        for (key, value) in [
            ("AUTH_INTERNAL_ENABLED", "yes"),
            ("AUTH_INTERNAL_ADDR", "127.0.0.1:0"),
            ("AUTH_INTERNAL_ALLOWED_CIDRS", "0.0.0.0/0"),
            ("AUTH_INTERNAL_ALLOWED_CIDRS", ""),
            ("AUTH_INTERNAL_BODY_LIMIT_BYTES", "65537"),
            ("AUTH_INTERNAL_PAIR_CAPACITY", "99"),
            ("AUTH_INTERNAL_LOCAL_LIMIT", "0"),
            ("AUTH_INTERNAL_SQL_TIMEOUT_MS", "1000"),
            ("AUTH_INTERNAL_LEASE_TTL_MS", "1999"),
        ] {
            let mut settings = values();
            settings.insert(key, value.into());
            assert!(parse(&settings).is_err(), "{key}");
        }
    }
}
