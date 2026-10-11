use anyhow::Context;
use std::env;

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
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        let server_addr = env::var("SERVER_ADDR").unwrap_or_else(|_| "0.0.0.0:3001".to_string());

        let database_url = required_env("DATABASE_URL")?;

        let database_max_connections = env::var("DATABASE_MAX_CONNECTIONS")
            .unwrap_or_else(|_| "10".to_string())
            .parse::<u32>()
            .context("DATABASE_MAX_CONNECTIONS must be a valid u32")?;

        Ok(Self {
            server_addr,
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
