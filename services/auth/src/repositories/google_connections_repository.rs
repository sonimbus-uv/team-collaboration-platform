use sqlx::{
    types::chrono::{DateTime, Utc},
    PgPool,
};
use uuid::Uuid;

use super::{
    federated_auth_repository::{FederatedAttempt, FederatedAuthRepository, NewFederatedAttempt},
    security_notifications_repository::SecurityNotificationsRepository,
    session_authentication_repository::{SessionActor, SessionAuthenticationRepository},
    sessions_repository::Tx,
};

pub enum LinkAdmission {
    Created(String),
    InvalidSession,
    StaleEvidence,
    InvalidTarget,
}

pub enum LinkResult {
    Linked,
    InvalidSession,
    StaleEvidence,
    InvalidAttempt,
    IdentityConflict,
    ProviderConflict,
    ContactConflict,
}

#[derive(sqlx::FromRow)]
pub struct ConnectionRow {
    pub provider: Option<String>,
    pub provider_email: Option<String>,
    pub linked_at: Option<DateTime<Utc>>,
}

pub struct GoogleConnectionsRepository;

impl GoogleConnectionsRepository {
    pub async fn create(
        pool: &PgPool,
        actor: SessionActor,
        attempt: NewFederatedAttempt<'_>,
    ) -> Result<LinkAdmission, sqlx::Error> {
        let mut tx = pool.begin().await?;
        let Some(session) = SessionAuthenticationRepository::lock(&mut tx, actor).await? else {
            return Ok(LinkAdmission::InvalidSession);
        };
        if !session.fresh {
            return Ok(LinkAdmission::StaleEvidence);
        }
        if session.client_type != attempt.client_type {
            return Ok(LinkAdmission::InvalidTarget);
        }

        let expires_at = sqlx::query_scalar(
            r#"
            INSERT INTO auth.federated_auth_attempts (
                id, purpose, client_type, device_name, redirect_target, initiator_secret_hash,
                launch_ticket_hash, oauth_state_hash, oidc_nonce_hash, code_challenge,
                initiator_account_id, initiator_session_id, initiator_authenticated_at,
                initiator_authentication_method, initiator_credential_version
            ) VALUES ($1, 'link', $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
            RETURNING to_char(expires_at AT TIME ZONE 'UTC', 'YYYY-MM-DD"T"HH24:MI:SS"Z"')
            "#,
        )
        .bind(attempt.id)
        .bind(attempt.client_type)
        .bind(session.device_name)
        .bind(attempt.redirect_target)
        .bind(attempt.secret_hash)
        .bind(attempt.ticket_hash)
        .bind(attempt.state_hash)
        .bind(attempt.nonce_hash)
        .bind(attempt.challenge)
        .bind(actor.account_id)
        .bind(actor.session_id)
        .bind(session.authenticated_at)
        .bind(session.method)
        .bind(session.credential_version)
        .fetch_one(&mut *tx)
        .await?;
        tx.commit().await?;

        Ok(LinkAdmission::Created(expires_at))
    }

    pub async fn list(
        pool: &PgPool,
        actor: SessionActor,
    ) -> Result<Vec<ConnectionRow>, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT i.provider, i.provider_email::text, i.linked_at
            FROM auth.session_account_status s
            LEFT JOIN auth.oauth_identities i ON i.account_id = s.account_id AND i.provider = 'google'
            WHERE s.id = $1 AND s.account_id = $2 AND s.account_allowed
                AND s.revoked_at IS NULL AND s.expires_at > clock_timestamp()
            "#,
        )
        .bind(actor.session_id)
        .bind(actor.account_id)
        .fetch_all(pool)
        .await
    }

    pub async fn link(
        tx: &mut Tx<'_>,
        attempt: &FederatedAttempt,
    ) -> Result<LinkResult, sqlx::Error> {
        FederatedAuthRepository::lock_provider_identity(tx, attempt).await?;

        let (Some(account_id), Some(session_id)) =
            (attempt.initiator_account_id, attempt.initiator_session_id)
        else {
            return Ok(LinkResult::InvalidAttempt);
        };

        let actor = SessionActor {
            account_id,
            session_id,
        };

        let Some(evidence) = SessionAuthenticationRepository::lock(tx, actor).await? else {
            return Ok(LinkResult::InvalidSession);
        };
        if !evidence.fresh
            || evidence.authenticated_at != attempt.initiator_authenticated_at
            || evidence.method != attempt.initiator_authentication_method
            || evidence.credential_version != attempt.initiator_credential_version
        {
            return Ok(LinkResult::StaleEvidence);
        }

        let existing: Option<(Uuid, Uuid)> = sqlx::query_as(
            r#"
            SELECT id, account_id
            FROM auth.oauth_identities
            WHERE provider = 'google'
            AND provider_subject = $1
            "#,
        )
        .bind(&attempt.provider_subject)
        .fetch_optional(&mut **tx)
        .await?;

        if existing.is_some_and(|(_, owner)| owner != account_id) {
            return Ok(LinkResult::IdentityConflict);
        }

        let new_link = existing.is_none();
        let identity_id = match existing {
            Some((identity_id, _)) => identity_id,
            None => {
                let has_provider = Self::has_google_identity(tx, account_id).await?;
                if has_provider {
                    return Ok(LinkResult::ProviderConflict);
                }
                let email_conflict =
                    Self::contact_email_in_use(tx, account_id, attempt.provider_email.as_deref())
                        .await?;
                if email_conflict {
                    return Ok(LinkResult::ContactConflict);
                }
                Self::insert_identity(tx, account_id, attempt).await?
            }
        };

        let event_id = Self::record_security_event(tx, account_id, identity_id, new_link).await?;
        let consumed =
            Self::consume_link_attempt(tx, attempt.id, account_id, identity_id, event_id).await?;
        Ok(if consumed {
            LinkResult::Linked
        } else {
            LinkResult::InvalidAttempt
        })
    }

    async fn has_google_identity(tx: &mut Tx<'_>, account_id: Uuid) -> Result<bool, sqlx::Error> {
        let has_provider: bool = sqlx::query_scalar(
            r#"
            SELECT EXISTS(
                SELECT 1 FROM auth.oauth_identities
                WHERE account_id = $1 AND provider = 'google'
            )
            "#,
        )
        .bind(account_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(has_provider)
    }

    async fn contact_email_in_use(
        tx: &mut Tx<'_>,
        account_id: Uuid,
        email: Option<&str>,
    ) -> Result<bool, sqlx::Error> {
        let email_conflict: bool = sqlx::query_scalar(
            "SELECT EXISTS(SELECT 1 FROM auth.accounts WHERE email = $1 AND id <> $2)",
        )
        .bind(email)
        .bind(account_id)
        .fetch_one(&mut **tx)
        .await?;

        Ok(email_conflict)
    }

    async fn insert_identity(
        tx: &mut Tx<'_>,
        account_id: Uuid,
        attempt: &FederatedAttempt,
    ) -> Result<Uuid, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            INSERT INTO auth.oauth_identities(account_id, provider, provider_subject, provider_email)
            VALUES ($1, 'google', $2, $3)
            RETURNING id
            "#,
        )
        .bind(account_id)
        .bind(&attempt.provider_subject)
        .bind(&attempt.provider_email)
        .fetch_one(&mut **tx)
        .await
    }

    async fn consume_link_attempt(
        tx: &mut Tx<'_>,
        attempt_id: Uuid,
        account_id: Uuid,
        identity_id: Uuid,
        event_id: Uuid,
    ) -> Result<bool, sqlx::Error> {
        let consumed = sqlx::query(
            r#"
            UPDATE auth.federated_auth_attempts SET status = 'consumed', terminal_at = clock_timestamp(),
                result_account_id = $2, result_identity_id = $3, result_security_event_id = $4
            WHERE id = $1 AND purpose = 'link' AND status = 'verified'
                AND expires_at > clock_timestamp() AND completion_expires_at > clock_timestamp()
                AND identity_valid_until > clock_timestamp()
                AND initiator_authenticated_at > clock_timestamp() - interval '10 minutes'
                AND EXISTS (SELECT 1 FROM auth.sessions s WHERE s.id = initiator_session_id
                    AND s.account_id = initiator_account_id AND s.revoked_at IS NULL
                    AND s.expires_at > clock_timestamp())
            "#,
        )
        .bind(attempt_id)
        .bind(account_id)
        .bind(identity_id)
        .bind(event_id)
        .execute(&mut **tx)
        .await?;

        Ok(consumed.rows_affected() == 1)
    }

    async fn record_security_event(
        tx: &mut Tx<'_>,
        account_id: Uuid,
        identity_id: Uuid,
        notify: bool,
    ) -> Result<Uuid, sqlx::Error> {
        let event_id: Uuid = sqlx::query_scalar(
            r#"
            INSERT INTO auth.account_security_events(account_id, oauth_identity_id) VALUES ($1, $2)
            ON CONFLICT (event_type, oauth_identity_id) DO UPDATE SET id = auth.account_security_events.id
            RETURNING id
            "#,
        )
        .bind(account_id)
        .bind(identity_id)
        .fetch_one(&mut **tx)
        .await?;

        if notify {
            SecurityNotificationsRepository::enqueue(tx, event_id, account_id).await?;
        }
        Ok(event_id)
    }
}
