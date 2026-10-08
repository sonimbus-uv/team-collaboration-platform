use super::accounts_repository::Account;
use sqlx::{types::Uuid, PgPool};

#[derive(Clone)]
pub struct OneTimeTokensRepository {
    pool: PgPool,
}

impl OneTimeTokensRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn reissue(&self, email: &str, hash: &[u8]) -> Result<bool, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let account_id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id
            FROM auth.accounts
            WHERE email = $1
              AND email_verified_at IS NULL
              AND status = 'active'
            FOR UPDATE
            "#,
        )
        .bind(email)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(id) = account_id else {
            return Ok(false);
        };

        let recent = sqlx::query_scalar::<_, bool>(
            r#"
            SELECT EXISTS (
                SELECT 1
                FROM auth.one_time_tokens
                WHERE account_id = $1
                  AND purpose = 'email_verification'
                  AND created_at > now() - interval '60 seconds'
            )
            "#,
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

        if recent {
            return Ok(false);
        }

        sqlx::query(
            r#"
            UPDATE auth.one_time_tokens
            SET invalidated_at = now()
            WHERE account_id = $1
              AND purpose = 'email_verification'
              AND used_at IS NULL
              AND invalidated_at IS NULL
            "#,
        )
        .bind(id)
        .execute(&mut *tx)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO auth.one_time_tokens (
                account_id,
                purpose,
                token_hash,
                sent_to_email,
                expires_at
            )
            VALUES (
                $1,
                'email_verification',
                $2,
                $3,
                now() + interval '24 hours'
            )
            "#,
        )
        .bind(id)
        .bind(hash)
        .bind(email)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(true)
    }

    pub async fn confirm(&self, hash: &[u8]) -> Result<Option<Account>, sqlx::Error> {
        let mut tx = self.pool.begin().await?;

        let id = sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT account_id
            FROM auth.one_time_tokens
            WHERE token_hash = $1
              AND purpose = 'email_verification'
            "#,
        )
        .bind(hash)
        .fetch_optional(&mut *tx)
        .await?;

        let Some(id) = id else {
            return Ok(None);
        };

        sqlx::query_scalar::<_, Uuid>(
            r#"
            SELECT id
            FROM auth.accounts
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

        let consumed = sqlx::query(
            r#"
            UPDATE auth.one_time_tokens t
            SET used_at = now()
            FROM auth.accounts a
            WHERE t.token_hash = $1
              AND t.account_id = a.id
              AND t.purpose = 'email_verification'
              AND t.used_at IS NULL
              AND t.invalidated_at IS NULL
              AND t.expires_at > clock_timestamp()
              AND t.sent_to_email = a.email
              AND a.status = 'active'
              AND a.email_verified_at IS NULL
            "#,
        )
        .bind(hash)
        .execute(&mut *tx)
        .await?;

        if consumed.rows_affected() != 1 {
            return Ok(None);
        }

        let account = sqlx::query_as::<_, Account>(
            r#"
            UPDATE auth.accounts
            SET email_verified_at = now()
            WHERE id = $1
            RETURNING
                id,
                email::TEXT AS email,
                status,
                email_verified_at IS NOT NULL AS email_verified
            "#,
        )
        .bind(id)
        .fetch_one(&mut *tx)
        .await?;

        tx.commit().await?;

        Ok(Some(account))
    }
}
