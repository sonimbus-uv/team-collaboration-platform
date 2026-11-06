use super::sessions_repository::{PasswordSnapshot, SessionsRepository};
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub type Tx<'a> = Transaction<'a, Postgres>;

#[derive(sqlx::FromRow)]
pub struct RecoveryRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub credential_version: i64,
    pub code_mac: Vec<u8>,
    pub usable: bool,
}

pub async fn password(
    pool: &PgPool,
    account_id: Uuid,
) -> Result<Option<PasswordSnapshot>, sqlx::Error> {
    sqlx::query_as(
        r#"
            SELECT
                account_id,
                password_hash,
                (EXTRACT(EPOCH FROM password_changed_at) * 1000000)::bigint AS version
            FROM auth.password_credentials
            WHERE account_id = $1
        "#,
    )
    .bind(account_id)
    .fetch_optional(pool)
    .await
}

pub async fn session_active(
    tx: &mut Tx<'_>,
    account_id: Uuid,
    session_id: Uuid,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(
        r#"
            SELECT EXISTS (
                SELECT 1
                FROM auth.sessions
                WHERE id = $1
                  AND account_id = $2
                  AND revoked_at IS NULL
                  AND expires_at > clock_timestamp()
            )
        "#,
    )
    .bind(session_id)
    .bind(account_id)
    .fetch_one(&mut **tx)
    .await
}

pub async fn recovery(
    pool: &PgPool,
    id: Uuid,
) -> Result<Option<RecoveryRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
            SELECT
                id,
                account_id,
                credential_version,
                code_mac,
                consumed_at IS NULL
                    AND invalidated_at IS NULL
                    AND failed_attempts < 5
                    AND expires_at > clock_timestamp() AS usable
            FROM auth.password_recovery_attempts
            WHERE id = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await
}

pub async fn lock_recovery(
    tx: &mut Tx<'_>,
    id: Uuid,
) -> Result<Option<RecoveryRow>, sqlx::Error> {
    sqlx::query_as(
        r#"
            SELECT
                id,
                account_id,
                credential_version,
                code_mac,
                consumed_at IS NULL
                    AND invalidated_at IS NULL
                    AND failed_attempts < 5
                    AND expires_at > clock_timestamp() AS usable
            FROM auth.password_recovery_attempts
            WHERE id = $1
            FOR UPDATE
        "#,
    )
    .bind(id)
    .fetch_optional(&mut **tx)
    .await
}

pub async fn invalidate(
    tx: &mut Tx<'_>,
    account_id: Uuid,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
            UPDATE auth.password_recovery_attempts
            SET invalidated_at = clock_timestamp()
            WHERE account_id = $1
              AND consumed_at IS NULL
              AND invalidated_at IS NULL
        "#,
    )
    .bind(account_id)
    .execute(&mut **tx)
    .await?;

    Ok(())
}

pub async fn install(
    tx: &mut Tx<'_>,
    snapshot: &PasswordSnapshot,
    password_hash: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query(
        r#"
            UPDATE auth.password_credentials
            SET password_hash = $2,
                password_changed_at = GREATEST(
                    clock_timestamp(),
                    password_changed_at + INTERVAL '1 microsecond'
                )
            WHERE account_id = $1
        "#,
    )
    .bind(snapshot.account_id)
    .bind(password_hash)
    .execute(&mut **tx)
    .await?;

    invalidate(tx, snapshot.account_id).await?;

    // Account locking serializes this deterministic session order with login/refresh/logout.
    let session_ids: Vec<Uuid> = sqlx::query_scalar(
        r#"
            SELECT id
            FROM auth.sessions
            WHERE account_id = $1
              AND revoked_at IS NULL
            ORDER BY id
            FOR UPDATE
        "#,
    )
    .bind(snapshot.account_id)
    .fetch_all(&mut **tx)
    .await?;

    for session_id in session_ids {
        SessionsRepository::revoke(tx, session_id, "password_changed").await?;
    }

    Ok(())
}
