use std::sync::Arc;

use sqlx::PgPool;

use crate::{
    auth_rate_limiter::AuthRateLimiter,
    services::{
        account_service::AuthService, email_verification_service::EmailVerificationService,
    },
};

#[derive(Clone)]
pub struct AppState {
    pub login_enabled: bool,
    pub token_issuer: Arc<auth_service::tokens::issuer::AccessTokenIssuer>,
    pub pool: PgPool,
    pub auth_service: AuthService,
    pub email_verification_service: EmailVerificationService,
    pub auth_rate_limiter: Arc<AuthRateLimiter>,
}

impl AppState {
    pub fn sessions(&self) -> crate::services::session_service::SessionService {
        crate::services::session_service::SessionService::new(
            crate::repositories::sessions_repository::SessionsRepository::new(self.pool.clone()),
            self.auth_service.clone(),
            self.token_issuer.clone(),
        )
    }
}
