mod config;
mod db;
mod dto;
mod email;
mod error;
mod handlers;
mod infrastructure;
mod internal;
mod ports;
mod protection;
mod providers;
mod repositories;
mod routes;
mod security;
mod services;
mod state;
#[cfg(test)]
mod tests;
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
    // Validate files/TLS and bind the private socket before starting either listener.
    let internal = match config.internal.clone() {
        Some(internal) => Some(
            internal::server::InternalServer::prepare(
                internal,
                config.redis_url.clone(),
                &config.database_url,
            )
            .await?,
        ),
        None => None,
    };
    let token_issuer = Arc::new(auth_service::tokens::issuer::AccessTokenIssuer::load(
        config.tokens.clone(),
    )?);
    let auth_rate_limiter = Arc::new(
        protection::auth_rate_limiter::AuthRateLimiter::from_config(&config)
            .map_err(|_| anyhow::anyhow!("Invalid Auth rate limiter configuration"))?,
    );

    let pool = db::connect(&config.database_url, config.database_max_connections).await?;
    db::run_migrations(&pool).await?;

    let accounts_repository = AccountsRepository::new(pool.clone());
    let password_hasher = Arc::new(Argon2PasswordHasher);
    let sender: Arc<dyn ports::email_verification::VerificationEmailSender> =
        match config.email_provider {
            config::EmailProvider::Resend { api_key } => Arc::new(
                providers::resend::ResendEmailSender::new(api_key, config.email_from)?,
            ),
            config::EmailProvider::Mailpit { host, port } => Arc::new(
                providers::mailpit::MailpitEmailSender::new(host, port, config.email_from)?,
            ),
        };

    let email_verification_service =
        services::email_verification_service::EmailVerificationService::new(
            repositories::registration_attempts_repository::RegistrationAttemptsRepository::new(
                pool.clone(),
            ),
            sender,
            config.registration_code_hmac_secret.as_bytes(),
        )?;
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
    let public = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    );
    if let Some(internal) = internal {
        tokio::select! {
            result = public => result?,
            result = internal.run() => result?,
        }
    } else {
        public.await?;
    }

    Ok(())
}
