use crate::protection::rate_limit::BucketPolicy;
use anyhow::Context;
use std::{env, net::IpAddr};

pub enum EmailProvider {
    Resend { api_key: String },
    Mailpit { host: String, port: u16 },
}

pub struct Config {
    pub contact_email_secret: Option<String>,
    pub security_notifications_enabled: bool,
    pub google: Option<crate::services::google_auth_config::GoogleAuthConfig>,
    pub password_recovery_secret: Option<String>,
    pub internal: Option<crate::internal::config::InternalConfig>,
    pub login_enabled: bool,
    pub google_ip_policy: BucketPolicy,
    pub login_ip_policy: BucketPolicy,
    pub refresh_ip_policy: BucketPolicy,
    pub logout_ip_policy: BucketPolicy,
    pub login_account_policy: BucketPolicy,
    pub tokens: auth_service::tokens::config::TokenConfig,
    pub server_addr: String,
    pub database_url: String,
    pub database_max_connections: u32,
    pub email_provider: EmailProvider,
    pub email_from: String,
    pub registration_code_hmac_secret: String,
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
        let server_addr = optional_string_env("SERVER_ADDR", "0.0.0.0:3001")?;

        let trusted_proxy_ips = match env::var("TRUSTED_PROXY_IPS") {
            Ok(value) => parse_trusted_proxy_ips(&value)?,
            Err(env::VarError::NotPresent) => Vec::new(),
            Err(env::VarError::NotUnicode(_)) => {
                anyhow::bail!("TRUSTED_PROXY_IPS must contain valid Unicode characters");
            }
        };

        let require_trusted_proxy = optional_bool_env("REQUIRE_TRUSTED_PROXY")?;

        anyhow::ensure!(
            !require_trusted_proxy || !trusted_proxy_ips.is_empty(),
            "TRUSTED_PROXY_IPS must not be empty when REQUIRE_TRUSTED_PROXY=true"
        );

        let recipient_hmac_secret = required_secret("AUTH_RECIPIENT_HMAC_SECRET")?;

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

        let database_max_connections = optional_u32_env("DATABASE_MAX_CONNECTIONS", 10)?;
        anyhow::ensure!(
            database_max_connections > 0,
            "DATABASE_MAX_CONNECTIONS must be greater than zero"
        );
        let registration_code_hmac_secret = required_secret("AUTH_REGISTRATION_CODE_HMAC_SECRET")?;
        let password_recovery_secret = if optional_bool_env("AUTH_PASSWORDS_ENABLED")? {
            let secret = required_secret("AUTH_PASSWORD_RESET_HMAC_SECRET")?;
            anyhow::ensure!(
                secret != recipient_hmac_secret && secret != registration_code_hmac_secret,
                "Password reset HMAC secret must be independent"
            );
            Some(secret)
        } else {
            None
        };
        let contact_email_secret = if optional_bool_env("AUTH_CONTACT_EMAIL_ENABLED")? {
            let secret = required_secret("AUTH_CONTACT_EMAIL_HMAC_SECRET")?;
            anyhow::ensure!(
                secret != registration_code_hmac_secret
                    && secret != recipient_hmac_secret
                    && password_recovery_secret
                        .as_ref()
                        .is_none_or(|other| other != &secret),
                "Contact email HMAC secret must be independent"
            );
            Some(secret)
        } else {
            None
        };
        let google = if optional_bool_env("AUTH_GOOGLE_ENABLED")? {
            Some(crate::services::google_auth_config::GoogleAuthConfig::new(
                required_env("AUTH_GOOGLE_CLIENT_ID")?,
                required_env("AUTH_GOOGLE_CLIENT_SECRET")?,
                &required_env("AUTH_GOOGLE_CALLBACK_URL")?,
                &required_env("AUTH_GOOGLE_RETURN_TARGETS")?,
            )?)
        } else {
            None
        };
        Ok(Self {
            contact_email_secret,
            security_notifications_enabled: optional_bool_env(
                "AUTH_SECURITY_NOTIFICATIONS_ENABLED",
            )?,
            google,
            password_recovery_secret,
            internal: crate::internal::config::InternalConfig::from_env()?,
            login_enabled: optional_bool_env("AUTH_LOGIN_ENABLED")?,
            google_ip_policy: bucket_policy_from_env("AUTH_GOOGLE_IP", 20, 60)?,
            login_ip_policy: bucket_policy_from_env("AUTH_LOGIN_IP", 6, 60)?,
            refresh_ip_policy: bucket_policy_from_env("AUTH_REFRESH_IP", 30, 60)?,
            logout_ip_policy: bucket_policy_from_env("AUTH_LOGOUT_IP", 30, 60)?,
            login_account_policy: bucket_policy_from_env("AUTH_LOGIN_ACCOUNT", 10, 900)?,
            tokens: auth_service::tokens::config::TokenConfig::from_env()?,
            server_addr,
            trusted_proxy_ips,
            recipient_hmac_secret,
            redis_url,
            redis_connect_timeout_ms,
            redis_operation_timeout_ms,
            register_ip_policy: bucket_policy_from_env("AUTH_REGISTER_IP", 6, 60)?,
            resend_ip_policy: bucket_policy_from_env("AUTH_RESEND_IP", 3, 60)?,
            verify_ip_policy: bucket_policy_from_env("AUTH_VERIFY_IP", 20, 60)?,
            recipient_policy: bucket_policy_from_env("AUTH_RECIPIENT", 10, 600)?,
            database_url,
            database_max_connections,
            email_provider: EmailProvider::from_env()?,
            email_from: required_env("EMAIL_FROM")?,
            registration_code_hmac_secret,
            max_concurrent_hashes: {
                let value = optional_string_env("MAX_CONCURRENT_HASHES", "4")?
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

impl EmailProvider {
    fn from_env() -> anyhow::Result<Self> {
        match optional_string_env("EMAIL_PROVIDER", "resend")?.as_str() {
            "resend" => Ok(Self::Resend {
                api_key: required_env("RESEND_API_KEY")?,
            }),
            "mailpit" => {
                let host = optional_string_env("MAILPIT_SMTP_HOST", "localhost")?;
                anyhow::ensure!(
                    !host.trim().is_empty(),
                    "MAILPIT_SMTP_HOST must not be empty"
                );
                let port = optional_string_env("MAILPIT_SMTP_PORT", "1025")?
                    .parse::<u16>()
                    .context("MAILPIT_SMTP_PORT must be a valid u16")?;
                anyhow::ensure!(port > 0, "MAILPIT_SMTP_PORT must be greater than zero");
                Ok(Self::Mailpit { host, port })
            }
            _ => anyhow::bail!("EMAIL_PROVIDER must be resend or mailpit"),
        }
    }
}

fn required_env(name: &str) -> anyhow::Result<String> {
    env::var(name)
        .map_err(|_| anyhow::anyhow!("Required environment variable {name} is missing or invalid"))
}

fn required_secret(name: &str) -> anyhow::Result<String> {
    let secret = required_env(name)?;
    anyhow::ensure!(
        secret.len() >= 32 && !secret.trim().is_empty(),
        "{name} must contain at least 32 bytes and must not be blank"
    );
    Ok(secret)
}

fn optional_string_env(name: &str, default: &str) -> anyhow::Result<String> {
    match env::var(name) {
        Ok(value) => Ok(value),
        Err(env::VarError::NotPresent) => Ok(default.to_owned()),
        Err(env::VarError::NotUnicode(_)) => anyhow::bail!("{name} must contain valid Unicode"),
    }
}

fn optional_bool_env(name: &str) -> anyhow::Result<bool> {
    parse_bool(name, &optional_string_env(name, "false")?)
}

fn parse_bool(name: &str, value: &str) -> anyhow::Result<bool> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => anyhow::bail!("{name} must be true or false"),
    }
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
    use super::{parse_bool, parse_trusted_proxy_ips};
    use std::net::IpAddr;

    #[test]
    fn booleans_are_strict_and_errors_do_not_echo_configuration_values() {
        assert!(parse_bool("FEATURE_ENABLED", "true").unwrap());
        assert!(!parse_bool("FEATURE_ENABLED", "false").unwrap());
        for value in [
            "",
            "TRUE",
            "1",
            " true",
            "false ",
            "sensitive-invalid-value",
        ] {
            let error = parse_bool("FEATURE_ENABLED", value).unwrap_err();
            assert_eq!(error.to_string(), "FEATURE_ENABLED must be true or false");
            assert!(!format!("{error:?}").contains("sensitive-invalid-value"));
        }
    }

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
