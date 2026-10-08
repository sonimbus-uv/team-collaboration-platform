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

#[derive(Debug)]
pub enum CreateAccountError {
    EmailAlreadyExists,
    Database(sqlx::Error),
}

impl AccountsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_with_password(
        &self,
        email: &str,
        password_hash: &str,
        token_hash: &[u8],
    ) -> Result<Account, CreateAccountError> {
        sqlx::query_as::<_, Account>(
            r#"
            WITH new_account AS (
                INSERT INTO auth.accounts (email)
                VALUES ($1)
                RETURNING id, email::TEXT AS email, status, email_verified_at IS NOT NULL AS email_verified
            ),
            new_password AS (
                INSERT INTO auth.password_credentials (account_id, password_hash)
                SELECT id, $2
                FROM new_account
            ), new_token AS (
                INSERT INTO auth.one_time_tokens (account_id, purpose, token_hash, sent_to_email, expires_at)
                SELECT id, 'email_verification', $3, email, now() + interval '24 hours'
                FROM new_account
            )
            SELECT id, email, status, email_verified
            FROM new_account
            "#,
        )
        .bind(email)
        .bind(password_hash)
        .bind(token_hash)
        .fetch_one(&self.pool)
        .await
        .map_err(map_create_account_error)
    }
}

fn map_create_account_error(err: sqlx::Error) -> CreateAccountError {
    if let sqlx::Error::Database(database_error) = &err {
        if database_error.constraint() == Some("accounts_email_unique") {
            return CreateAccountError::EmailAlreadyExists;
        }
    }

    CreateAccountError::Database(err)
}
