use crate::session_check_types::SessionCheckRequest;
use sqlx::{
    types::chrono::{DateTime, Utc},
    PgPool,
};
use uuid::Uuid;

#[derive(Debug, sqlx::FromRow)]
pub(crate) struct SessionCheckRow {
    pub sid: Uuid,
    pub account_id: Uuid,
    pub account_allowed: bool,
    pub unrevoked: bool,
    pub expires_at: Option<DateTime<Utc>>,
    pub observed_at: DateTime<Utc>,
}

#[derive(Clone)]
pub(crate) struct SessionChecksRepository {
    pool: PgPool,
}

impl SessionChecksRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn check(
        &self,
        requests: &[SessionCheckRequest],
    ) -> Result<Vec<SessionCheckRow>, sqlx::Error> {
        let sids: Vec<_> = requests.iter().map(|r| r.sid).collect();
        let accounts: Vec<_> = requests.iter().map(|r| r.account_id).collect();
        // One statement / MVCC snapshot, no row locks. Missing pairs remain in the result.
        // statement_timestamp() supplies one database clock value for the entire batch.
        sqlx::query_as(
            r#"
            SELECT input.sid, input.account_id,
                   COALESCE(a.status = 'active' AND a.email_verified_at IS NOT NULL, false)
                       AS account_allowed,
                   s.id IS NOT NULL AND s.revoked_at IS NULL AS unrevoked,
                   s.expires_at, statement_timestamp() AS observed_at
            FROM unnest($1::uuid[], $2::uuid[]) WITH ORDINALITY
                AS input(sid, account_id, position)
            LEFT JOIN auth.sessions s ON s.id = input.sid AND s.account_id = input.account_id
            LEFT JOIN auth.accounts a ON a.id = s.account_id
            ORDER BY input.position
            "#,
        )
        .bind(sids)
        .bind(accounts)
        .fetch_all(&self.pool)
        .await
    }
}
