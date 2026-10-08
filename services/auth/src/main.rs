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
mod repositories;
mod routes;
mod security;
mod services;
mod state;
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
    let pool = db::connect(&config.database_url, config.database_max_connections).await?;

    db::run_migrations(&pool).await?;

    let accounts_repository = AccountsRepository::new(pool.clone());
    let password_hasher = Arc::new(Argon2PasswordHasher);
    let sender = Arc::new(providers::resend::ResendEmailSender::new(
        config.resend_api_key,
        config.email_from,
        config.email_verification_url,
    )?);
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
        pool,
        auth_service,
        email_verification_service,
    };

    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(config.server_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
