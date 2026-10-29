use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

pub type Tx<'a> = Transaction<'a, Postgres>;

#[derive(sqlx::FromRow)]
pub struct PasswordSnapshot {
    pub account_id: Uuid,
    pub password_hash: String,
    pub version: i64,
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
            SELECT a.id AS account_id, p.password_hash,
                   (extract(epoch FROM p.password_changed_at)*1000000)::bigint AS version
            FROM auth.accounts a
            JOIN auth.password_credentials p ON p.account_id=a.id
            WHERE a.email=$1
        "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn lock_account(tx: &mut Tx<'_>, id: Uuid) -> Result<Option<bool>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT status='active' AND email_verified_at IS NOT NULL
            FROM auth.accounts WHERE id=$1 FOR UPDATE
        "#,
        )
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
            SELECT password_hash, (extract(epoch FROM password_changed_at)*1000000)::bigint
            FROM auth.password_credentials WHERE account_id=$1 FOR UPDATE
        "#,
        )
        .bind(snapshot.account_id)
        .fetch_optional(&mut **tx)
        .await?;
        Ok(current.is_some_and(|(hash, version)| {
            hash == snapshot.password_hash && version == snapshot.version
        }))
    }

    pub async fn open(
        tx: &mut Tx<'_>,
        account: Uuid,
        client: &str,
        device: Option<&str>,
    ) -> Result<SessionRow, sqlx::Error> {
        let id: Uuid = sqlx::query_scalar("SELECT auth.open_session($1,$2,interval '20 days',$3)")
            .bind(account)
            .bind(client)
            .bind(device)
            .fetch_one(&mut **tx)
            .await?;
        Self::lock(tx, id).await
    }

    pub async fn lock(tx: &mut Tx<'_>, id: Uuid) -> Result<SessionRow, sqlx::Error> {
        sqlx::query_as(r#"
            SELECT id, account_id,
                   floor(extract(epoch FROM created_at))::bigint AS login_at,
                   floor(extract(epoch FROM expires_at))::bigint AS expires_at,
                   to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS expires_text,
                   revoked_at IS NULL AND expires_at>clock_timestamp() AS usable
            FROM auth.sessions WHERE id=$1 FOR UPDATE
        "#)
            .bind(id).fetch_one(&mut **tx).await
    }

    pub async fn revoke(tx: &mut Tx<'_>, id: Uuid, reason: &str) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE auth.sessions SET revoked_at=clock_timestamp(), revoke_reason=$2 WHERE id=$1 AND revoked_at IS NULL")
            .bind(id).bind(reason).execute(&mut **tx).await?;
        Ok(())
    }
}
