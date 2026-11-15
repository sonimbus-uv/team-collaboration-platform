use sqlx::{
    types::chrono::{DateTime, Utc},
    PgPool, Postgres, Transaction,
};
use uuid::Uuid;

pub type Tx<'a> = Transaction<'a, Postgres>;

#[derive(sqlx::FromRow)]
pub struct PasswordSnapshot {
    pub account_id: Uuid,
    pub password_hash: String,
    pub version: i64,
}

pub struct PasswordSessionContext<'a> {
    pub credentials: &'a PasswordSnapshot,
    pub client_type: &'a str,
    pub device_name: Option<&'a str>,
    pub authenticated_at: DateTime<Utc>,
}

#[derive(sqlx::FromRow)]
pub struct SessionRow {
    pub id: Uuid,
    pub account_id: Uuid,
    pub login_at: i64,
    pub expires_at: i64,
    pub expires_text: String,
    pub usable: bool,
}

#[derive(Clone)]
pub struct SessionsRepository {
    pub pool: PgPool,
}

impl SessionsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn password(&self, email: &str) -> Result<Option<PasswordSnapshot>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT
                a.id AS account_id,
                p.password_hash,
                (EXTRACT(EPOCH FROM p.password_changed_at) * 1000000)::bigint AS version
            FROM auth.accounts a
            JOIN auth.password_credentials p ON p.account_id = a.id
            WHERE a.email = $1
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn lock_password_account(
        tx: &mut Tx<'_>,
        id: Uuid,
    ) -> Result<Option<bool>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT status = 'active' AND email_verified_at IS NOT NULL
            FROM auth.accounts
            WHERE id = $1
            FOR UPDATE
            "#,
        )
        .bind(id)
        .fetch_optional(&mut **tx)
        .await
    }

    pub async fn lock_active_account(
        tx: &mut Tx<'_>,
        id: Uuid,
    ) -> Result<Option<bool>, sqlx::Error> {
        sqlx::query_scalar("SELECT status = 'active' FROM auth.accounts WHERE id = $1 FOR UPDATE")
            .bind(id)
            .fetch_optional(&mut **tx)
            .await
    }

    pub async fn password_unchanged(
        tx: &mut Tx<'_>,
        snapshot: &PasswordSnapshot,
    ) -> Result<bool, sqlx::Error> {
        let current: Option<(String, i64)> = sqlx::query_as(
            r#"
            SELECT
                password_hash,
                (EXTRACT(EPOCH FROM password_changed_at) * 1000000)::bigint
            FROM auth.password_credentials
            WHERE account_id = $1
            FOR UPDATE
            "#,
        )
        .bind(snapshot.account_id)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(current.is_some_and(|(hash, version)| {
            hash == snapshot.password_hash && version == snapshot.version
        }))
    }

    pub async fn open_password(
        tx: &mut Tx<'_>,
        context: PasswordSessionContext<'_>,
    ) -> Result<SessionRow, sqlx::Error> {
        let id: Uuid = sqlx::query_scalar(
            r#"
            SELECT auth.open_authenticated_session($1, $2, interval '20 days',
                ROW('password', NULL, $4, $5)::auth.session_authentication_context, $3)
            "#,
        )
        .bind(context.credentials.account_id)
        .bind(context.client_type)
        .bind(context.device_name)
        .bind(context.credentials.version)
        .bind(context.authenticated_at)
        .fetch_one(&mut **tx)
        .await?;
        Self::lock(tx, id).await
    }

    pub async fn lock(tx: &mut Tx<'_>, id: Uuid) -> Result<SessionRow, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT
                s.id,
                s.account_id,
                FLOOR(EXTRACT(EPOCH FROM s.created_at))::bigint AS login_at,
                FLOOR(EXTRACT(EPOCH FROM s.expires_at))::bigint AS expires_at,
                to_char(s.expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS expires_text,
                s.revoked_at IS NULL AND s.expires_at > clock_timestamp()
                    AND eligibility.account_allowed AS usable
            FROM auth.sessions s
            JOIN auth.session_account_status eligibility ON eligibility.id = s.id
            WHERE s.id = $1
            FOR UPDATE OF s
            "#,
        )
        .bind(id)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn revoke(tx: &mut Tx<'_>, id: Uuid, reason: &str) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE auth.sessions
            SET revoked_at = clock_timestamp(), revoke_reason = $2
            WHERE id = $1
              AND revoked_at IS NULL
            "#,
        )
        .bind(id)
        .bind(reason)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
