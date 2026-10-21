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
    pub pool: PgPool,
    pub auth_service: AuthService,
    pub email_verification_service: EmailVerificationService,
    pub auth_rate_limiter: Arc<AuthRateLimiter>,
}
