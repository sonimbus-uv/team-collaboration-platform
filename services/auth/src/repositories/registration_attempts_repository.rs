//! Registration codes only. Legacy one_time_tokens cannot activate accounts.
use super::accounts_repository::Account;
use crate::security::registration_code::RegistrationCodeKey;
use sqlx::{
    types::chrono::{DateTime, Utc},
    PgPool, Postgres, Transaction,
};
use uuid::Uuid;

pub enum RegistrationError {
    Invalid,
    Limited(u64),
    AlreadyRegistered,
    Database(sqlx::Error),
    Unavailable,
}
impl std::fmt::Debug for RegistrationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Invalid => "Invalid",
            Self::Limited(_) => "Limited",
            Self::AlreadyRegistered => "AlreadyRegistered",
            Self::Database(_) => "Database([REDACTED])",
            Self::Unavailable => "Unavailable",
        })
    }
}
impl From<sqlx::Error> for RegistrationError {
    fn from(value: sqlx::Error) -> Self {
        Self::Database(value)
    }
}

#[derive(sqlx::FromRow)]
pub struct RegistrationAttempt {
    pub id: Uuid,
    pub email: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
struct LockedAttempt {
    password_hash: String,
    code_mac: Vec<u8>,
    code_version: i32,
    usable: bool,
    resend_wait: i64,
}

#[derive(Clone)]
pub struct RegistrationAttemptsRepository {
    pool: PgPool,
}

impl RegistrationAttemptsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // All writers: email advisory lock -> account row (if present) -> attempt row.
    // The advisory lock also serializes the absence of an account before INSERT.
    async fn lock_email(
        tx: &mut Transaction<'_, Postgres>,
        email: &str,
    ) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 17005))")
            .bind(email)
            .execute(&mut **tx)
            .await?;
        sqlx::query("SELECT id FROM auth.accounts WHERE email=$1 FOR UPDATE")
            .bind(email)
            .fetch_optional(&mut **tx)
            .await?;
        Ok(())
    }

    async fn registered(
        tx: &mut Transaction<'_, Postgres>,
        email: &str,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT EXISTS(SELECT 1
            FROM auth.accounts
            WHERE email=$1
            AND (email_verified_at IS NOT NULL OR status <> 'active'))
            "#,
        )
        .bind(email)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn create(
        &self,
        email: &str,
        password_hash: &str,
        id: Uuid,
        mac: &[u8],
    ) -> Result<RegistrationAttempt, RegistrationError> {
        let mut tx = self.pool.begin().await?;
        Self::lock_email(&mut tx, email).await?;
        if Self::registered(&mut tx, email).await? {
            return Err(RegistrationError::AlreadyRegistered);
        }
        let pending: i64 = sqlx::query_scalar(
            r#"
            SELECT count(*)
            FROM auth.registration_attempts
            WHERE email=$1
            AND consumed_at IS NULL
            AND invalidated_at IS NULL
            AND expires_at>clock_timestamp()
            AND failed_attempts<5
            "#,
        )
        .bind(email)
        .fetch_one(&mut *tx)
        .await?;
        if pending >= 5 {
            return Err(RegistrationError::Limited(600));
        }
        let attempt = sqlx::query_as(
            r#"
            INSERT INTO auth.registration_attempts(id,email,password_hash,code_mac)
            VALUES($1,$2,$3,$4)
            RETURNING id,email::text AS email,expires_at
            "#,
        )
        .bind(id)
        .bind(email)
        .bind(password_hash)
        .bind(mac)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(attempt)
    }

    pub async fn find(&self, id: Uuid) -> Result<RegistrationAttempt, RegistrationError> {
        sqlx::query_as(
            "SELECT id,email::text AS email,expires_at FROM auth.registration_attempts WHERE id=$1",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?
        .ok_or(RegistrationError::Invalid)
    }

    pub async fn resend_candidate(
        &self,
        id: Uuid,
    ) -> Result<RegistrationAttempt, RegistrationError> {
        let attempt = self.find(id).await?;
        let (usable, version, wait): (bool, i32, i64) = sqlx::query_as(r#"
            SELECT consumed_at IS NULL
            AND invalidated_at IS NULL
            AND expires_at>clock_timestamp()
            AND failed_attempts<5
            AND NOT EXISTS(SELECT 1
            FROM auth.accounts a
            WHERE a.email=t.email
            AND (a.email_verified_at IS NOT NULL OR a.status<>'active')), code_version, GREATEST(0,ceil(extract(epoch
            FROM last_sent_at+interval '60 seconds'-clock_timestamp())))::bigint
            FROM auth.registration_attempts t
            WHERE id=$1
            "#)
            .bind(id).fetch_one(&self.pool).await?;
        if !usable {
            return Err(RegistrationError::Invalid);
        }
        if version >= 3 {
            return Err(RegistrationError::Limited(600));
        }
        if wait > 0 {
            return Err(RegistrationError::Limited(wait as u64));
        }
        Ok(attempt)
    }

    async fn lock_attempt(
        tx: &mut Transaction<'_, Postgres>,
        id: Uuid,
    ) -> Result<LockedAttempt, RegistrationError> {
        sqlx::query_as(
            r#"
            WITH locked AS MATERIALIZED (
                SELECT * FROM auth.registration_attempts WHERE id=$1 FOR UPDATE
            )
            SELECT password_hash,code_mac,code_version, consumed_at IS NULL
            AND invalidated_at IS NULL
            AND expires_at>clock_timestamp()
            AND failed_attempts<5 AS usable, GREATEST(0,ceil(extract(epoch
            FROM last_sent_at+interval '60 seconds'-clock_timestamp())))::bigint AS resend_wait
            FROM locked
            "#,
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await?
        .ok_or(RegistrationError::Invalid)
    }

    pub async fn confirm(
        &self,
        id: Uuid,
        code: &str,
        key: &RegistrationCodeKey,
    ) -> Result<Account, RegistrationError> {
        let attempt = self.find(id).await?;
        let mut tx = self.pool.begin().await?;
        Self::lock_email(&mut tx, &attempt.email).await?;
        let row = Self::lock_attempt(&mut tx, id).await?;
        if !row.usable || Self::registered(&mut tx, &attempt.email).await? {
            return Err(RegistrationError::Invalid);
        }
        if !key.verify(id, row.code_version, code, &row.code_mac) {
            sqlx::query("UPDATE auth.registration_attempts SET failed_attempts=failed_attempts+1 WHERE id=$1")
                .bind(id).execute(&mut *tx).await?;
            // Persist the failed attempt before returning the HTTP-domain error.
            tx.commit().await?;
            return Err(RegistrationError::Invalid);
        }
        let account: Account = sqlx::query_as(
            r#"
            INSERT INTO auth.accounts(email,email_verified_at)
            VALUES($1,clock_timestamp())
            ON CONFLICT (email)
            DO UPDATE SET email_verified_at=clock_timestamp()
            WHERE auth.accounts.email_verified_at IS NULL
            AND auth.accounts.status='active'
            RETURNING id,email::text AS email,status,email_verified_at IS NOT NULL AS email_verified
            "#,
        )
        .bind(&attempt.email)
        .fetch_optional(&mut *tx)
        .await?
        .ok_or(RegistrationError::Invalid)?;
        sqlx::query(
            r#"
            INSERT INTO auth.password_credentials(account_id,password_hash)
            VALUES($1,$2)
            ON CONFLICT(account_id)
            DO UPDATE SET password_hash=EXCLUDED.password_hash,password_changed_at=clock_timestamp()
            "#,
        )
        .bind(account.id)
        .bind(&row.password_hash)
        .execute(&mut *tx)
        .await?;
        sqlx::query(
            r#"
            UPDATE auth.sessions SET revoked_at=clock_timestamp(),revoke_reason='password_changed'
            WHERE account_id=$1
            AND revoked_at IS NULL
            "#,
        )
        .bind(account.id)
        .execute(&mut *tx)
        .await?;
        sqlx::query(r#"
            UPDATE auth.registration_attempts SET consumed_at=CASE WHEN id=$1 THEN clock_timestamp() ELSE consumed_at END,invalidated_at=CASE WHEN id<>$1 THEN clock_timestamp() ELSE invalidated_at END
            WHERE email=$2
            AND consumed_at IS NULL
            AND invalidated_at IS NULL
            "#)
            .bind(id).bind(&attempt.email).execute(&mut *tx).await?;
        sqlx::query(
            r#"
            UPDATE auth.one_time_tokens SET invalidated_at=clock_timestamp()
            WHERE account_id=$1
            AND purpose='email_verification'
            AND used_at IS NULL
            AND invalidated_at IS NULL
            "#,
        )
        .bind(account.id)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(account)
    }

    pub async fn reissue(
        &self,
        id: Uuid,
        key: &RegistrationCodeKey,
        code: &mut zeroize::Zeroizing<String>,
    ) -> Result<RegistrationAttempt, RegistrationError> {
        let attempt = self.find(id).await?;
        let mut tx = self.pool.begin().await?;
        Self::lock_email(&mut tx, &attempt.email).await?;
        let row = Self::lock_attempt(&mut tx, id).await?;
        if !row.usable || Self::registered(&mut tx, &attempt.email).await? {
            return Err(RegistrationError::Invalid);
        }
        if row.code_version >= 3 {
            return Err(RegistrationError::Limited(600));
        }
        if row.resend_wait > 0 {
            return Err(RegistrationError::Limited(row.resend_wait as u64));
        }
        // Do not accidentally reissue the same numeric value under a new version.
        while key.verify(id, row.code_version, code, &row.code_mac) {
            *code = RegistrationCodeKey::generate();
        }
        let mac = key.digest(id, row.code_version + 1, code);
        sqlx::query(r#"
            UPDATE auth.registration_attempts SET code_version=code_version+1,code_mac=$2,last_sent_at=clock_timestamp()
            WHERE id=$1
            "#)
            .bind(id).bind(mac).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(attempt)
    }
}
