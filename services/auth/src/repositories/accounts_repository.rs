use sqlx::{types::Uuid, PgPool};

#[derive(Clone)]
pub struct AccountsRepository {
    pool: PgPool,
}

#[derive(Debug, sqlx::FromRow)]
pub struct Account {
    pub id: Uuid,
    pub email: String,
    pub status: String,
    pub email_verified: bool,
}

pub enum CreateAccountError {
    EmailAlreadyExists,
    Database(sqlx::Error),
}

impl std::fmt::Debug for CreateAccountError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::EmailAlreadyExists => "EmailAlreadyExists",
            Self::Database(_) => "Database([REDACTED])",
        })
    }
}

impl AccountsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
    pub async fn is_registered(&self, email: &str) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM auth.accounts WHERE email=$1 AND (email_verified_at IS NOT NULL OR status<>'active'))")
            .bind(email).fetch_one(&self.pool).await
    }
}
