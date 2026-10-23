mod auth_rate_limiter;
mod client_ip;
mod config;
mod db;
mod dto;
mod email;
mod error;
mod handlers;
#[cfg(test)]
mod integration_tests;
mod ports;
mod providers;
mod rate_limit;
mod recipient_key;
mod redis_connection;
mod repositories;
mod routes;
mod security;
mod services;
#[cfg(test)]
mod session_tests;
mod state;
#[cfg(test)]
mod token_test_support;
mod validators;

use std::sync::Arc;

use repositories::accounts_repository::AccountsRepository;
use security::password_hasher::Argon2PasswordHasher;
use services::account_service::AuthService;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(anyhow::anyhow!("Failed to load .env: {error}")),
    }
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "auth_service=info".into()),
        )
        .init();

    let config = config::Config::from_env()?;
    // Fail before opening databases or serving requests; never generate a fallback key.
    let token_issuer = Arc::new(auth_service::tokens::issuer::AccessTokenIssuer::load(
        config.tokens.clone(),
    )?);
    let auth_rate_limiter = Arc::new(
        auth_rate_limiter::AuthRateLimiter::from_config(&config)
            .map_err(|_| anyhow::anyhow!("Invalid Auth rate limiter configuration"))?,
    );
    let pool = db::connect(&config.database_url, config.database_max_connections).await?;

    db::run_migrations(&pool).await?;

    let accounts_repository = AccountsRepository::new(pool.clone());
    let password_hasher = Arc::new(Argon2PasswordHasher);
    let sender: Arc<dyn ports::email_verification::VerificationEmailSender> =
        match config.email_provider {
            config::EmailProvider::Resend { api_key } => {
                Arc::new(providers::resend::ResendEmailSender::new(
                    api_key,
                    config.email_from,
                    config.email_verification_url,
                )?)
            }
            config::EmailProvider::Mailpit { host, port } => {
                Arc::new(providers::mailpit::MailpitEmailSender::new(
                    host,
                    port,
                    config.email_from,
                    config.email_verification_url,
                )?)
            }
        };
    let email_verification_service =
        services::email_verification_service::EmailVerificationService::new(
            repositories::one_time_tokens_repository::OneTimeTokensRepository::new(pool.clone()),
            sender,
        );
    let auth_service = AuthService::new(
        accounts_repository,
        password_hasher,
        email_verification_service.clone(),
        config.max_concurrent_hashes,
    );

    let state = AppState {
        login_enabled: config.login_enabled,
        token_issuer,
        pool,
        auth_service,
        email_verification_service,
        auth_rate_limiter,
    };

    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(config.server_addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    )
    .await?;

    Ok(())
}
