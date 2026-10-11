use sqlx::PgPool;

use crate::services::{
    account_service::AuthService, email_verification_service::EmailVerificationService,
};

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub auth_service: AuthService,
    pub email_verification_service: EmailVerificationService,
}
