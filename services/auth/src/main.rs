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

use repositories::{
    accounts_repository::AccountsRepository, sessions_repository::SessionsRepository,
};
use security::password_hasher::Argon2PasswordHasher;
use services::account_service::AuthService;
use state::AppState;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(_) => {
            return Err(anyhow::anyhow!(
                "Failed to load .env; check its syntax and permissions"
            ))
        }
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

    let pool = db::connect(&config.database_url, config.database_max_connections)
        .await
        .map_err(|_| {
            anyhow::anyhow!("Database connection failed; check configuration and availability")
        })?;
    db::run_migrations(&pool)
        .await
        .map_err(|_| anyhow::anyhow!("Database migrations failed; check schema and permissions"))?;

    let server_addr = config.server_addr.clone();
    let state = build_state(config, pool, token_issuer, auth_rate_limiter)?;
    let cleanup_services = services::maintenance::CleanupServices {
        passwords: state.password_service.clone(),
        google: state.google_service.clone(),
        contact_email: state.contact_email_service.clone(),
    };
    let notification_service = state.security_notifications.clone();
    let app = routes::router(state);
    let listener = tokio::net::TcpListener::bind(server_addr).await?;
    let public = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
    );
    let cleanup_task = services::maintenance::spawn_cleanup(cleanup_services);
    let notification_task = notification_service.map(|service| service.spawn());
    let result = if let Some(internal) = internal {
        tokio::select! {
            result = public => result.map_err(anyhow::Error::from),
            result = internal.run() => result,
        }
    } else {
        public.await.map_err(anyhow::Error::from)
    };
    cleanup_task.abort();
    let _ = cleanup_task.await;
    if let Some(task) = notification_task {
        task.abort();
        let _ = task.await;
    }
    result
}

fn build_state(
    config: config::Config,
    pool: sqlx::PgPool,
    token_issuer: Arc<auth_service::tokens::issuer::AccessTokenIssuer>,
    auth_rate_limiter: Arc<protection::auth_rate_limiter::AuthRateLimiter>,
) -> anyhow::Result<AppState> {
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
            sender.clone(),
            config.registration_code_hmac_secret.as_bytes(),
        )?;
    let password_processor = services::password_processor::PasswordProcessor::new(
        password_hasher,
        config.max_concurrent_hashes,
    );
    let auth_service = AuthService::new(
        accounts_repository,
        password_processor.clone(),
        email_verification_service.clone(),
    );

    let password_service = config
        .password_recovery_secret
        .as_ref()
        .map(|secret| {
            services::password_service::PasswordService::new(
                crate::services::password_service::PasswordServiceDependencies {
                    pool: pool.clone(),
                    passwords: password_processor.clone(),
                    sender: sender.clone(),
                    limiter: auth_rate_limiter.clone(),
                },
                secret.as_bytes(),
            )
        })
        .transpose()?;
    let google_service = config
        .google
        .map(|google| {
            services::google_auth_service::GoogleAuthService::new(
                repositories::federated_auth_repository::FederatedAuthRepository {
                    pool: pool.clone(),
                },
                services::session_service::SessionService::new(
                    SessionsRepository::new(pool.clone()),
                    password_processor.clone(),
                    token_issuer.clone(),
                ),
                google,
            )
        })
        .transpose()?;
    let contact_email_service = config
        .contact_email_secret
        .as_ref()
        .map(|secret| {
            services::contact_email_service::ContactEmailService::new(
                repositories::contact_email_repository::ContactEmailRepository {
                    pool: pool.clone(),
                },
                sender.clone(),
                auth_rate_limiter.clone(),
                secret.as_bytes(),
            )
        })
        .transpose()?;
    let security_notifications = config.security_notifications_enabled.then(|| {
        services::security_notification_service::SecurityNotificationService::new(
            repositories::security_notifications_repository::SecurityNotificationsRepository {
                pool: pool.clone(),
            },
            sender,
        )
    });
    Ok(AppState {
        contact_email_service,
        security_notifications,
        google_service,
        password_service,
        login_enabled: config.login_enabled,
        token_issuer,
        pool,
        auth_service,
        email_verification_service,
        auth_rate_limiter,
    })
}
