mod config;
mod db;
mod routes;
mod state;
mod error;

use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenvy::dotenv().ok();

    let config = config::Config::from_env()?;
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
        pool,
        auth_service,
        email_verification_service,
    };

    let app = routes::router(state);

    let listener = tokio::net::TcpListener::bind(config.server_addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
