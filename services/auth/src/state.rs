use crate::{
    protection::auth_rate_limiter::AuthRateLimiter,
    repositories::sessions_repository::SessionsRepository,
    services::{
        account_service::AuthService, email_verification_service::EmailVerificationService,
        password_service::PasswordService, session_service::SessionService,
    },
};
use auth_service::tokens::issuer::AccessTokenIssuer;
use sqlx::PgPool;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub password_service: Option<PasswordService>,
    pub login_enabled: bool,
    pub token_issuer: Arc<AccessTokenIssuer>,
    pub pool: PgPool,
    pub auth_service: AuthService,
    pub email_verification_service: EmailVerificationService,
    pub auth_rate_limiter: Arc<AuthRateLimiter>,
}

impl AppState {
    pub fn sessions(&self) -> SessionService {
        SessionService::new(
            SessionsRepository::new(self.pool.clone()),
            self.auth_service.password_processor(),
            self.token_issuer.clone(),
        )
    }
}
