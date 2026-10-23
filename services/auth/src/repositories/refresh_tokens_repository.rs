use super::sessions_repository::{SessionRow, Tx};
use sqlx::PgPool;
use uuid::Uuid;

#[derive(sqlx::FromRow)]
pub struct RefreshRow {
    pub id: Uuid,
    pub consumed: bool,
    pub usable: bool,
}

pub struct RefreshTokensRepository;
impl RefreshTokensRepository {
    // Identity lookup takes no locks; all authoritative checks happen after account/session locks.
    pub async fn identity(pool: &PgPool, hash: &[u8]) -> Result<Option<(Uuid, Uuid)>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT s.account_id, s.id
            FROM auth.refresh_tokens r JOIN auth.sessions s ON s.id=r.session_id
            WHERE r.token_hash=$1
        "#,
        )
        .bind(hash)
        .fetch_optional(pool)
        .await
    }

    pub async fn lock(
        tx: &mut Tx<'_>,
        hash: &[u8],
        session: Uuid,
    ) -> Result<RefreshRow, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, replaced_by_token_id IS NOT NULL AS consumed,
                   revoked_at IS NULL AND expires_at>clock_timestamp() AS usable
            FROM auth.refresh_tokens WHERE token_hash=$1 AND session_id=$2 FOR UPDATE
        "#,
        )
        .bind(hash)
        .bind(session)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn insert(
        tx: &mut Tx<'_>,
        session: &SessionRow,
        hash: &[u8],
    ) -> Result<Uuid, sqlx::Error> {
        sqlx::query_scalar("INSERT INTO auth.refresh_tokens(session_id,token_hash,expires_at) VALUES($1,$2,to_timestamp($3::double precision)) RETURNING id")
            .bind(session.id).bind(hash).bind(session.expires_at as f64).fetch_one(&mut **tx).await
    }

    pub async fn consume(tx: &mut Tx<'_>, id: Uuid, next: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query("UPDATE auth.refresh_tokens SET revoked_at=clock_timestamp(), replaced_by_token_id=$2 WHERE id=$1")
            .bind(id).bind(next).execute(&mut **tx).await?;
        Ok(())
    }
}
