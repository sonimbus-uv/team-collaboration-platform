use sqlx::{
    types::chrono::{DateTime, Utc},
    PgPool,
};
use uuid::Uuid;

use super::sessions_repository::{PasswordSnapshot, SessionsRepository, Tx};

#[derive(Clone, Copy)]
pub struct SessionActor {
    pub account_id: Uuid,
    pub session_id: Uuid,
}

#[derive(sqlx::FromRow)]
pub struct SessionAuthentication {
    pub client_type: String,
    pub device_name: Option<String>,
    pub authenticated_at: Option<DateTime<Utc>>,
    pub method: Option<String>,
    pub credential_version: Option<i64>,
    pub fresh: bool,
}

pub struct SessionAuthenticationRepository;

impl SessionAuthenticationRepository {
    pub async fn password(
        pool: &PgPool,
        actor: SessionActor,
    ) -> Result<Option<PasswordSnapshot>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT p.account_id, p.password_hash,
                (EXTRACT(EPOCH FROM p.password_changed_at) * 1000000)::bigint AS version
            FROM auth.password_credentials p
            JOIN auth.accounts a ON a.id = p.account_id
            JOIN auth.session_account_status s ON s.account_id = a.id
            WHERE a.id = $1 AND s.id = $2 AND s.account_allowed
                AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp()
                AND a.email_verified_at IS NOT NULL
            "#,
        )
        .bind(actor.account_id)
        .bind(actor.session_id)
        .fetch_optional(pool)
        .await
    }

    pub async fn lock(
        tx: &mut Tx<'_>,
        actor: SessionActor,
    ) -> Result<Option<SessionAuthentication>, sqlx::Error> {
        if SessionsRepository::lock_active_account(tx, actor.account_id).await? != Some(true) {
            return Ok(None);
        }
        // Match the account -> password credentials -> session lock order.
        let password_version = Self::lock_password_version(tx, actor.account_id).await?;

        sqlx::query_as(
            r#"
            SELECT s.client_type, s.device_name,
                s.last_authenticated_at AS authenticated_at,
                s.reauthentication_method AS method,
                s.reauthentication_password_version AS credential_version,
                COALESCE(s.last_authenticated_at > clock_timestamp() - interval '10 minutes'
                    AND s.last_authenticated_at <= clock_timestamp()
                    AND ((s.reauthentication_method = 'password'
                            AND s.reauthentication_password_version = $3)
                        OR (s.reauthentication_method = 'google'
                            AND s.reauthentication_oauth_identity_id IS NOT NULL)), false) AS fresh
            FROM auth.sessions s
            JOIN auth.session_account_status eligibility ON eligibility.id = s.id
            WHERE s.account_id = $1 AND s.id = $2 AND eligibility.account_allowed
                AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp()
            FOR UPDATE OF s
            "#,
        )
        .bind(actor.account_id)
        .bind(actor.session_id)
        .bind(password_version)
        .fetch_optional(&mut **tx)
        .await
    }

    async fn lock_password_version(
        tx: &mut Tx<'_>,
        account_id: Uuid,
    ) -> Result<Option<i64>, sqlx::Error> {
        let password_version: Option<i64> = sqlx::query_scalar(
            r#"
            SELECT (EXTRACT(EPOCH FROM password_changed_at) * 1000000)::bigint
            FROM auth.password_credentials WHERE account_id = $1 FOR UPDATE
            "#,
        )
        .bind(account_id)
        .fetch_optional(&mut **tx)
        .await?;

        Ok(password_version)
    }

    pub async fn record_password_proof(
        tx: &mut Tx<'_>,
        actor: SessionActor,
        proof: &PasswordSnapshot,
        authenticated_at: DateTime<Utc>,
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE auth.sessions SET last_authenticated_at = $4,
                reauthentication_method = 'password', reauthentication_password_version = $3,
                reauthentication_oauth_identity_id = NULL
            WHERE account_id = $1 AND id = $2
                AND (last_authenticated_at IS NULL OR last_authenticated_at <= $4)
            "#,
        )
        .bind(actor.account_id)
        .bind(actor.session_id)
        .bind(proof.version)
        .bind(authenticated_at)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }
}
