use crate::rate_limit::BucketPolicy;
use anyhow::Context;
use std::{env, net::IpAddr};

pub enum EmailProvider {
    Resend { api_key: String },
    Mailpit { host: String, port: u16 },
}

pub struct Config {
    pub server_addr: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub email_provider: EmailProvider,
    pub email_from: String,
    pub email_verification_url: String,
    pub max_concurrent_hashes: usize,
    pub trusted_proxy_ips: Vec<IpAddr>,
    pub register_ip_policy: BucketPolicy,
    pub resend_ip_policy: BucketPolicy,
    pub verify_ip_policy: BucketPolicy,
    pub recipient_policy: BucketPolicy,
    pub recipient_hmac_secret: String,
    pub redis_url: String,
    pub redis_connect_timeout_ms: u32,
    pub redis_operation_timeout_ms: u32,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:3001".to_string());

        let trusted_proxy_ips = match env::var("TRUSTED_PROXY_IPS") {
            Ok(value) => parse_trusted_proxy_ips(&value)?,
            Err(env::VarError::NotPresent) => Vec::new(),
            Err(env::VarError::NotUnicode(_)) => {
                anyhow::bail!("TRUSTED_PROXY_IPS must contain valid Unicode characters");
            }
        };

        let require_trusted_proxy = match env::var("REQUIRE_TRUSTED_PROXY") {
            Ok(value) if value == "true" => true,
            Ok(value) if value == "false" => false,
            Err(env::VarError::NotPresent) => false,
            _ => anyhow::bail!("REQUIRE_TRUSTED_PROXY must be true or false"),
        };

        anyhow::ensure!(
            !require_trusted_proxy || !trusted_proxy_ips.is_empty(),
            "TRUSTED_PROXY_IPS must not be empty when REQUIRE_TRUSTED_PROXY=true"
        );

        let recipient_hmac_secret = required_env("AUTH_RECIPIENT_HMAC_SECRET")?;

        anyhow::ensure!(
            recipient_hmac_secret.len() >= 32 && !recipient_hmac_secret.trim().is_empty(),
            "AUTH_RECIPIENT_HMAC_SECRET must contain at least 32 bytes and must not be blank"
        );

        let redis_url = required_env("REDIS_URL")?;
        anyhow::ensure!(!redis_url.trim().is_empty(), "REDIS_URL must not be empty");

        let redis_connect_timeout_ms = optional_u32_env("REDIS_CONNECT_TIMEOUT_MS", 500)?;
        let redis_operation_timeout_ms = optional_u32_env("REDIS_OPERATION_TIMEOUT_MS", 200)?;

        anyhow::ensure!(
            (1..=5_000).contains(&redis_connect_timeout_ms),
            "REDIS_CONNECT_TIMEOUT_MS must be between 1 and 5000"
        );
        anyhow::ensure!(
            (1..=5_000).contains(&redis_operation_timeout_ms),
            "REDIS_OPERATION_TIMEOUT_MS must be between 1 and 5000"
        );

        let database_url = required_env("DATABASE_URL")?;

        let database_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .context("DATABASE_MAX_CONNECTIONS must be a valid u32")?;

        Ok(Self {
            server_addr,
            trusted_proxy_ips,
            recipient_hmac_secret,
            redis_url,
            redis_connect_timeout_ms,
            redis_operation_timeout_ms,
            register_ip_policy: bucket_policy_from_env("AUTH_REGISTER_IP", 6, 60)?,
            resend_ip_policy: bucket_policy_from_env("AUTH_RESEND_IP", 3, 60)?,
            verify_ip_policy: bucket_policy_from_env("AUTH_VERIFY_IP", 20, 60)?,
            recipient_policy: bucket_policy_from_env("AUTH_RECIPIENT", 3, 3600)?,
            database_url,
            database_max_connections,
            email_provider: match env::var("EMAIL_PROVIDER")
                .unwrap_or_else(|_| "resend".into())
                .as_str()
            {
                "resend" => EmailProvider::Resend {
                    api_key: required_env("RESEND_API_KEY")?,
                },
                "mailpit" => {
                    let host = env::var("MAILPIT_SMTP_HOST").unwrap_or_else(|_| "localhost".into());
                    anyhow::ensure!(
                        !host.trim().is_empty(),
                        "MAILPIT_SMTP_HOST must not be empty"
                    );
                    let port = env::var("MAILPIT_SMTP_PORT")
                        .unwrap_or_else(|_| "1025".into())
                        .parse::<u16>()
                        .context("MAILPIT_SMTP_PORT must be a valid u16")?;
                    anyhow::ensure!(port > 0, "MAILPIT_SMTP_PORT must be greater than zero");
                    EmailProvider::Mailpit { host, port }
                }
                _ => anyhow::bail!("EMAIL_PROVIDER must be resend or mailpit"),
            },
            email_from: required_env("EMAIL_FROM")?,
            email_verification_url: required_env("EMAIL_VERIFICATION_URL")?,
            max_concurrent_hashes: {
                let value = env::var("MAX_CONCURRENT_HASHES")
                    .unwrap_or_else(|_| "4".into())
                    .parse::<usize>()
                    .context("MAX_CONCURRENT_HASHES must be a valid positive integer")?;
                anyhow::ensure!(
                    value > 0 && value <= 64,
                    "MAX_CONCURRENT_HASHES must be between 1 and 64"
                );
                value
            },
        })
    }
}

fn required_env(name: &str) -> anyhow::Result<String> {
    env::var(name)
        .with_context(|| format!("Required environment variable {name} is missing or invalid"))
}

fn parse_trusted_proxy_ips(value: &str) -> anyhow::Result<Vec<IpAddr>> {
    if value.trim().is_empty() {
        return Ok(Vec::new());
    }

    value
        .split(',')
        .map(|entry| {
            entry
                .trim()
                .parse::<IpAddr>()
                .context("TRUSTED_PROXY_IPS must be a comma-separated list of IP addresses without ports or CIDR")
        })
        .collect()
}

fn bucket_policy_from_env(
    prefix: &str,
    default_capacity: u32,
    default_refill_period_secs: u32,
) -> anyhow::Result<BucketPolicy> {
    let capacity_name = format!("{prefix}_CAPACITY");
    let refill_name = format!("{prefix}_REFILL_SECS");

    let capacity = optional_u32_env(&capacity_name, default_capacity)?;
    let refill_period_secs = optional_u32_env(&refill_name, default_refill_period_secs)?;

    BucketPolicy::new(capacity, refill_period_secs)
        .with_context(|| format!("Invalid bucket policy for {prefix}"))
}

fn optional_u32_env(name: &str, default: u32) -> anyhow::Result<u32> {
    match env::var(name) {
        Ok(value) => value
            .parse::<u32>()
            .with_context(|| format!("{name} must be a valid unsigned integer")),
        Err(env::VarError::NotPresent) => Ok(default),
        Err(env::VarError::NotUnicode(_)) => {
            anyhow::bail!("{name} must contain valid Unicode");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_trusted_proxy_ips;
    use std::net::IpAddr;

    #[test]
    fn missing_entries_mean_no_trusted_proxies() {
        assert!(parse_trusted_proxy_ips("").unwrap().is_empty());
        assert!(parse_trusted_proxy_ips("   ").unwrap().is_empty());
    }

    #[test]
    fn accepts_ipv4_and_ipv6_with_surrounding_spaces() {
        let result = parse_trusted_proxy_ips(" 192.0.2.20, 2001:db8::20 ").unwrap();

        let expected: Vec<IpAddr> = vec![
            "192.0.2.20".parse().unwrap(),
            "2001:db8::20".parse().unwrap(),
        ];

        assert_eq!(result, expected);
    }

    #[test]
    fn rejects_invalid_entries_ports_networks_and_empty_elements() {
        for value in [
            "not-an-ip",
            "192.0.2.20:3001",
            "192.0.2.0/24",
            "192.0.2.20,",
            ",192.0.2.20",
            "192.0.2.20,,192.0.2.21",
        ] {
            assert!(
                parse_trusted_proxy_ips(value).is_err(),
                "unexpectedly accepted {value:?}"
            );
        }
    }
}
