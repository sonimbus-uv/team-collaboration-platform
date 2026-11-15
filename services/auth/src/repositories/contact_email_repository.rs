use sqlx::{
    types::chrono::{DateTime, Utc},
    PgPool,
};
use uuid::Uuid;

use super::{
    session_authentication_repository::{SessionActor, SessionAuthenticationRepository},
    sessions_repository::Tx,
};
use crate::{
    dto::contact_email::{ContactEmail, ContactVerificationAccepted},
    security::contact_email_code::ContactCodeContext,
};

#[derive(sqlx::FromRow)]
pub struct ContactOwner {
    pub email: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct ContactPreflight {
    pub email: Option<String>,
    pub in_use: bool,
    pub live_id: Option<Uuid>,
    pub live_expires: Option<String>,
}

#[derive(sqlx::FromRow)]
pub struct ContactResendCandidate {
    pub email: String,
    pub code_version: i32,
    pub resend_wait: i64,
}

#[derive(sqlx::FromRow)]
pub struct ContactAttempt {
    pub id: Uuid,
    pub account_id: Uuid,
    pub email: String,
    pub contact_email_version: i64,
    pub code_version: i32,
    pub code_mac: Vec<u8>,
    pub consumed: bool,
    pub usable: bool,
    pub resend_wait: i64,
    pub expires_at: DateTime<Utc>,
}

impl ContactAttempt {
    pub fn context(&self) -> ContactCodeContext<'_> {
        ContactCodeContext {
            account_id: self.account_id,
            email: &self.email,
            attempt_id: self.id,
            contact_version: self.contact_email_version,
            code_version: self.code_version,
        }
    }

    pub fn accepted(&self) -> ContactVerificationAccepted {
        ContactVerificationAccepted {
            verification_attempt_id: self.id,
            expires_at: self.expires_at.format("%Y-%m-%dT%H:%M:%SZ").to_string(),
        }
    }
}

pub struct NewContactAttempt<'a> {
    pub context: ContactCodeContext<'a>,
    pub code_mac: &'a [u8],
}

#[derive(Clone)]
pub struct ContactEmailRepository {
    pub pool: PgPool,
}

impl ContactEmailRepository {
    pub async fn preflight(
        &self,
        actor: SessionActor,
        email: &str,
    ) -> Result<Option<ContactPreflight>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT a.email::text,
                EXISTS(SELECT 1 FROM auth.accounts owner WHERE owner.email = $3 AND owner.id <> a.id) AS in_use,
                t.id AS live_id,
                to_char(t.expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS live_expires
            FROM auth.session_account_status s JOIN auth.accounts a ON a.id = s.account_id
            LEFT JOIN auth.account_email_proposals p ON p.account_id = a.id
            LEFT JOIN auth.contact_email_verification_attempts t ON t.account_id = a.id
                AND t.email = $3 AND t.contact_email_version = p.version AND t.email = p.email
                AND t.consumed_at IS NULL AND t.invalidated_at IS NULL
                AND t.failed_attempts < 5 AND t.expires_at > clock_timestamp()
            WHERE s.account_id = $1 AND s.id = $2 AND s.account_allowed
                AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp()
            "#,
        )
        .bind(actor.account_id)
        .bind(actor.session_id)
        .bind(email)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn resend_candidate(
        &self,
        actor: SessionActor,
        attempt_id: Uuid,
    ) -> Result<Option<ContactResendCandidate>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT t.email::text, t.code_version,
                GREATEST(0, CEIL(EXTRACT(EPOCH FROM t.last_sent_at + interval '60 seconds' - clock_timestamp())))::bigint AS resend_wait
            FROM auth.contact_email_verification_attempts t
            JOIN auth.account_email_proposals p ON p.account_id = t.account_id AND p.version = t.contact_email_version AND p.email = t.email
            JOIN auth.session_account_status s ON s.account_id = t.account_id AND s.id = $3
            WHERE t.id = $1 AND t.account_id = $2 AND s.account_allowed
                AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp()
                AND t.consumed_at IS NULL AND t.invalidated_at IS NULL
                AND t.failed_attempts < 5 AND t.expires_at > clock_timestamp()
            "#,
        )
        .bind(attempt_id)
        .bind(actor.account_id)
        .bind(actor.session_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn snapshot(&self, actor: SessionActor) -> Result<Option<ContactEmail>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT a.email::text AS contact_email, a.email_verified_at IS NOT NULL AS contact_email_verified,
                p.email::text AS pending_contact_email
            FROM auth.session_account_status s JOIN auth.accounts a ON a.id = s.account_id
            LEFT JOIN auth.account_email_proposals p ON p.account_id = a.id
            WHERE s.id = $1 AND s.account_id = $2 AND s.account_allowed
                AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp()
            "#,
        )
        .bind(actor.session_id)
        .bind(actor.account_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn lock_owner(
        tx: &mut Tx<'_>,
        actor: SessionActor,
    ) -> Result<Option<ContactOwner>, sqlx::Error> {
        if SessionAuthenticationRepository::lock(tx, actor)
            .await?
            .is_none()
        {
            return Ok(None);
        }
        sqlx::query_as("SELECT email::text FROM auth.accounts WHERE id = $1")
            .bind(actor.account_id)
            .fetch_optional(&mut **tx)
            .await
    }

    pub async fn candidate(
        &self,
        actor: SessionActor,
        attempt_id: Uuid,
    ) -> Result<Option<String>, sqlx::Error> {
        sqlx::query_scalar("SELECT email::text FROM auth.contact_email_verification_attempts WHERE id = $1 AND account_id = $2")
            .bind(attempt_id)
        .bind(actor.account_id)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn lock_email(tx: &mut Tx<'_>, email: &str) -> Result<(), sqlx::Error> {
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 17005))")
            .bind(email)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    pub async fn in_use(
        tx: &mut Tx<'_>,
        account_id: Uuid,
        email: &str,
    ) -> Result<bool, sqlx::Error> {
        sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM auth.accounts WHERE email = $1 AND id <> $2)",
        )
        .bind(email)
        .bind(account_id)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn propose(
        tx: &mut Tx<'_>,
        account_id: Uuid,
        email: &str,
    ) -> Result<i64, sqlx::Error> {
        let proposal_exists: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM auth.account_email_proposals WHERE account_id = $1)",
        )
        .bind(account_id)
        .fetch_one(&mut **tx)
        .await?;

        if proposal_exists {
            sqlx::query("UPDATE auth.account_email_proposals SET email = $2 WHERE account_id = $1")
                .bind(account_id)
                .bind(email)
                .execute(&mut **tx)
                .await?;
        } else {
            sqlx::query(
                "INSERT INTO auth.account_email_proposals(account_id, email) VALUES ($1, $2)",
            )
            .bind(account_id)
            .bind(email)
            .execute(&mut **tx)
            .await?;
        }
        sqlx::query_scalar("SELECT version FROM auth.account_email_proposals WHERE account_id = $1")
            .bind(account_id)
            .fetch_one(&mut **tx)
            .await
    }

    pub async fn lock_attempt(
        tx: &mut Tx<'_>,
        actor: SessionActor,
        attempt_id: Option<Uuid>,
    ) -> Result<Option<ContactAttempt>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT t.id, t.account_id, t.email::text, t.contact_email_version, t.code_version, t.code_mac,
                t.consumed_at IS NOT NULL AS consumed, t.expires_at,
                t.consumed_at IS NULL AND t.invalidated_at IS NULL AND t.failed_attempts < 5
                    AND t.expires_at > clock_timestamp()
                    AND EXISTS(SELECT 1 FROM auth.account_email_proposals p WHERE p.account_id = t.account_id
                        AND p.email = t.email AND p.version = t.contact_email_version) AS usable,
                GREATEST(0, CEIL(EXTRACT(EPOCH FROM t.last_sent_at + interval '60 seconds' - clock_timestamp())))::bigint AS resend_wait
            FROM auth.contact_email_verification_attempts t
            WHERE t.account_id = $1
                AND (
                    ($2::uuid IS NOT NULL AND t.id = $2)
                    OR ($2::uuid IS NULL AND t.consumed_at IS NULL AND t.invalidated_at IS NULL)
                )
            FOR UPDATE OF t
        "#
        )
            .bind(actor.account_id)
            .bind(attempt_id)
            .fetch_optional(&mut **tx)
            .await
    }

    pub async fn invalidate(tx: &mut Tx<'_>, account_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE auth.contact_email_verification_attempts
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

    pub async fn create(
        tx: &mut Tx<'_>,
        attempt: NewContactAttempt<'_>,
    ) -> Result<ContactVerificationAccepted, sqlx::Error> {
        sqlx::query_as(
            r#"
            INSERT INTO auth.contact_email_verification_attempts(id, account_id, email, contact_email_version, code_mac)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id AS verification_attempt_id, to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"') AS expires_at
            "#,
        )
        .bind(attempt.context.attempt_id)
        .bind(attempt.context.account_id)
        .bind(attempt.context.email)
        .bind(attempt.context.contact_version)
        .bind(attempt.code_mac)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn replace_code(
        tx: &mut Tx<'_>,
        attempt_id: Uuid,
        code_mac: &[u8],
    ) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE auth.contact_email_verification_attempts
            SET code_mac = $2, code_version = code_version + 1, last_sent_at = clock_timestamp()
            WHERE id = $1
            "#,
        )
        .bind(attempt_id)
        .bind(code_mac)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn fail(tx: &mut Tx<'_>, attempt_id: Uuid) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE auth.contact_email_verification_attempts
            SET failed_attempts = failed_attempts + 1
            WHERE id = $1
            AND failed_attempts < 5
            "#,
        )
        .bind(attempt_id)
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    pub async fn assign(
        tx: &mut Tx<'_>,
        actor: SessionActor,
        attempt: &ContactAttempt,
    ) -> Result<bool, sqlx::Error> {
        let account_update = sqlx::query(
            r#"
            UPDATE auth.accounts
            SET email = $2, email_verified_at = clock_timestamp()
            WHERE id = $1
            AND email IS NULL
            "#,
        )
        .bind(actor.account_id)
        .bind(&attempt.email)
        .execute(&mut **tx)
        .await?;

        if account_update.rows_affected() != 1 {
            return Ok(false);
        }

        let attempt_update = sqlx::query(
            r#"
            UPDATE auth.contact_email_verification_attempts
            SET consumed_at = clock_timestamp()
            WHERE id = $1
            AND expires_at > clock_timestamp()
            AND consumed_at IS NULL
            AND invalidated_at IS NULL
            AND failed_attempts < 5
            AND EXISTS(SELECT 1 FROM auth.sessions s WHERE s.id = $2 AND s.account_id = $3
                AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp())
            "#,
        )
        .bind(attempt.id)
        .bind(actor.session_id)
        .bind(actor.account_id)
        .execute(&mut **tx)
        .await?;

        if attempt_update.rows_affected() != 1 {
            return Ok(false);
        }
        sqlx::query("DELETE FROM auth.account_email_proposals WHERE account_id = $1")
            .bind(actor.account_id)
            .execute(&mut **tx)
            .await?;

        Ok(true)
    }

    pub async fn cleanup(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE
            FROM auth.contact_email_verification_attempts
            WHERE id IN (SELECT id
            FROM auth.contact_email_verification_attempts
            WHERE expires_at < clock_timestamp() - interval '24 hours'
            ORDER BY expires_at
            LIMIT 1000)
            "#,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
