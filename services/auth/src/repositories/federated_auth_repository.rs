use sqlx::{
    types::chrono::{DateTime, Utc},
    PgPool,
};
use uuid::Uuid;

use super::sessions_repository::Tx;

#[derive(sqlx::FromRow)]
pub struct FederatedAttempt {
    pub id: Uuid,
    pub purpose: String,
    pub initiator_account_id: Option<Uuid>,
    pub initiator_session_id: Option<Uuid>,
    pub initiator_authenticated_at: Option<DateTime<Utc>>,
    pub initiator_authentication_method: Option<String>,
    pub initiator_credential_version: Option<i64>,
    pub redirect_target: String,
    pub client_type: String,
    pub device_name: Option<String>,
    pub initiator_secret_hash: Vec<u8>,
    pub oidc_nonce_hash: Vec<u8>,
    pub code_challenge: Vec<u8>,
    pub completion_code_hash: Option<Vec<u8>>,
    pub provider_subject: Option<String>,
    pub provider_email: Option<String>,
    pub email_authoritative: bool,
    pub identity_verified_at: Option<DateTime<Utc>>,
    pub usable: bool,
}

pub struct NewFederatedAttempt<'a> {
    pub id: Uuid,
    pub client_type: &'a str,
    pub device_name: Option<&'a str>,
    pub redirect_target: &'a str,
    pub secret_hash: &'a [u8],
    pub ticket_hash: &'a [u8],
    pub state_hash: &'a [u8],
    pub nonce_hash: &'a [u8],
    pub challenge: &'a [u8],
}

pub struct FederatedProof<'a> {
    pub subject: &'a str,
    pub email: Option<&'a str>,
    pub email_authoritative: bool,
    pub valid_until: DateTime<Utc>,
    pub completion_hash: &'a [u8],
}

pub struct CompletedGoogleLogin {
    pub account_id: Uuid,
    pub identity_id: Uuid,
    pub session_id: Uuid,
}

pub enum IdentityResolution {
    Allowed { account_id: Uuid, identity_id: Uuid },
    ContactConflict,
    AccountBlocked,
}

#[derive(Clone)]
pub struct FederatedAuthRepository {
    pub pool: PgPool,
}

impl FederatedAuthRepository {
    pub async fn create(&self, attempt: NewFederatedAttempt<'_>) -> Result<String, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            INSERT INTO auth.federated_auth_attempts(id,
                purpose,
                client_type,
                device_name,
                redirect_target,
                initiator_secret_hash,
                launch_ticket_hash,
                oauth_state_hash,
                oidc_nonce_hash,
                code_challenge)
            VALUES ($1,
                'login',
                $2,
                $3,
                $4,
                $5,
                $6,
                $7,
                $8,
                $9)
            RETURNING to_char(expires_at AT TIME ZONE 'UTC',
                'YYYY-MM-DD"T"HH24:MI:SS"Z"')
            "#,
        )
        .bind(attempt.id)
        .bind(attempt.client_type)
        .bind(attempt.device_name)
        .bind(attempt.redirect_target)
        .bind(attempt.secret_hash)
        .bind(attempt.ticket_hash)
        .bind(attempt.state_hash)
        .bind(attempt.nonce_hash)
        .bind(attempt.challenge)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn authorize(&self, ticket_hash: &[u8]) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            UPDATE auth.federated_auth_attempts
            SET status = 'authorizing',
                authorization_started_at = clock_timestamp()
            WHERE launch_ticket_hash = $1
            AND status = 'created'
            AND expires_at > clock_timestamp()
            RETURNING id
            "#,
        )
        .bind(ticket_hash)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn claim_callback(&self, state_hash: &[u8]) -> Result<Option<Uuid>, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            INSERT INTO auth.federated_callback_claims(attempt_id) SELECT id
            FROM auth.federated_auth_attempts
            WHERE oauth_state_hash = $1
            AND status = 'authorizing'
            AND expires_at > clock_timestamp()
            ON CONFLICT
            DO NOTHING
            RETURNING attempt_id
            "#,
        )
        .bind(state_hash)
        .fetch_optional(&self.pool)
        .await
    }

    pub async fn find(&self, attempt_id: Uuid) -> Result<FederatedAttempt, sqlx::Error> {
        sqlx::query_as(
            r#"
            SELECT id, purpose, initiator_account_id, initiator_session_id,
                initiator_authenticated_at, initiator_authentication_method, initiator_credential_version,
                redirect_target,
                client_type,
                device_name,
                initiator_secret_hash,
                oidc_nonce_hash,
                code_challenge,
                completion_code_hash,
                provider_subject,
                provider_email::text,
                email_authoritative,
                identity_verified_at,
                status = 'verified'
            AND expires_at > clock_timestamp()
            AND completion_expires_at > clock_timestamp()
            AND identity_valid_until > clock_timestamp()
            AS usable
            FROM auth.federated_auth_attempts
            WHERE id = $1
            "#,
        )
        .bind(attempt_id)
        .fetch_one(&self.pool)
        .await
    }

    pub async fn fail(&self, attempt_id: Uuid, cancelled: bool) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            UPDATE auth.federated_auth_attempts
            SET status = $2,
                terminal_at = clock_timestamp()
            WHERE id = $1
            AND status = 'authorizing'
            "#,
        )
        .bind(attempt_id)
        .bind(if cancelled { "cancelled" } else { "failed" })
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn record_proof(
        &self,
        attempt_id: Uuid,
        proof: FederatedProof<'_>,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            WITH observed AS (SELECT clock_timestamp() AS instant) UPDATE auth.federated_auth_attempts
            SET status = 'verified',
                provider_subject = $2,
                provider_email = $3,
                email_authoritative = $4,
                identity_verified_at = observed.instant,
                identity_valid_until = $5,
                completion_code_hash = $6,
                completion_issued_at = observed.instant,
                completion_expires_at = LEAST(observed.instant + interval '60 seconds',
                expires_at,
                $5)
            FROM observed
            WHERE id = $1
            AND status = 'authorizing'
            AND expires_at > observed.instant
            AND $5 > observed.instant
            "#,
        )
            .bind(attempt_id)
        .bind(proof.subject)
        .bind(proof.email)
        .bind(proof.email_authoritative)
        .bind(proof.valid_until)
        .bind(proof.completion_hash)
            .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn resolve_identity(
        tx: &mut Tx<'_>,
        attempt: &FederatedAttempt,
    ) -> Result<IdentityResolution, sqlx::Error> {
        Self::lock_provider_identity(tx, attempt).await?;

        let existing: Option<(Uuid, Uuid, bool)> = sqlx::query_as(
            r#"
            SELECT a.id,
                i.id,
                a.status = 'active'
            FROM auth.oauth_identities i
            JOIN auth.accounts a ON a.id = i.account_id
            WHERE i.provider = 'google'
            AND i.provider_subject = $1
            FOR UPDATE OF a
            "#,
        )
        .bind(&attempt.provider_subject)
        .fetch_optional(&mut **tx)
        .await?;

        if let Some((account_id, identity_id, active)) = existing {
            return Ok(if active {
                IdentityResolution::Allowed {
                    account_id,
                    identity_id,
                }
            } else {
                IdentityResolution::AccountBlocked
            });
        }

        let account_id = Self::create_account(tx, attempt).await?;
        let Some(account_id) = account_id else {
            return Ok(IdentityResolution::ContactConflict);
        };

        let identity_id = sqlx::query_scalar(
            r#"
            INSERT INTO auth.oauth_identities(account_id,
                provider,
                provider_subject,
                provider_email)
            VALUES ($1,
                'google',
                $2,
                $3)
            RETURNING id
            "#,
        )
        .bind(account_id)
        .bind(&attempt.provider_subject)
        .bind(&attempt.provider_email)
        .fetch_one(&mut **tx)
        .await?;
        Self::propose_unverified_email(tx, account_id, attempt).await?;

        Ok(IdentityResolution::Allowed {
            account_id,
            identity_id,
        })
    }

    async fn create_account(
        tx: &mut Tx<'_>,
        attempt: &FederatedAttempt,
    ) -> Result<Option<Uuid>, sqlx::Error> {
        let account_id = if attempt.email_authoritative {
            sqlx::query_scalar(
                r#"
            INSERT INTO auth.accounts(email,
                email_verified_at)
            VALUES ($1,
                clock_timestamp())
            ON CONFLICT (email)
            DO NOTHING
            RETURNING id
            "#,
            )
            .bind(&attempt.provider_email)
            .fetch_optional(&mut **tx)
            .await?
        } else {
            Some(
                sqlx::query_scalar("INSERT INTO auth.accounts DEFAULT VALUES RETURNING id")
                    .fetch_one(&mut **tx)
                    .await?,
            )
        };
        Ok(account_id)
    }

    async fn propose_unverified_email(
        tx: &mut Tx<'_>,
        account_id: Uuid,
        attempt: &FederatedAttempt,
    ) -> Result<(), sqlx::Error> {
        if attempt.email_authoritative {
            return Ok(());
        }
        let Some(email) = &attempt.provider_email else {
            return Ok(());
        };

        sqlx::query("INSERT INTO auth.account_email_proposals(account_id, email) VALUES ($1, $2)")
            .bind(account_id)
            .bind(email)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    pub(super) async fn lock_provider_identity(
        tx: &mut Tx<'_>,
        attempt: &FederatedAttempt,
    ) -> Result<(), sqlx::Error> {
        // Keep email and provider locks in this order for login and linking.
        if let Some(email) = &attempt.provider_email {
            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 17005))")
                .bind(email)
                .execute(&mut **tx)
                .await?;
        }
        sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1, 17009))")
            .bind(&attempt.provider_subject)
            .execute(&mut **tx)
            .await?;

        Ok(())
    }

    pub async fn open_session(
        tx: &mut Tx<'_>,
        attempt: &FederatedAttempt,
        account_id: Uuid,
        identity_id: Uuid,
    ) -> Result<Uuid, sqlx::Error> {
        sqlx::query_scalar(
            r#"
            SELECT auth.open_authenticated_session($1,
                $2,
                interval '20 days',
                ROW('google',
                $3,
                NULL,
                $4)::auth.session_authentication_context,
                $5)
            "#,
        )
        .bind(account_id)
        .bind(&attempt.client_type)
        .bind(identity_id)
        .bind(attempt.identity_verified_at)
        .bind(&attempt.device_name)
        .fetch_one(&mut **tx)
        .await
    }

    pub async fn consume(
        tx: &mut Tx<'_>,
        attempt_id: Uuid,
        completed_login: CompletedGoogleLogin,
    ) -> Result<bool, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE auth.federated_auth_attempts
            SET status = 'consumed',
                terminal_at = clock_timestamp(),
                result_account_id = $2,
                result_identity_id = $3,
                result_session_id = $4
            WHERE id = $1
            AND status = 'verified'
            AND purpose = 'login'
            AND completion_expires_at > clock_timestamp()
            AND expires_at > clock_timestamp()
            AND identity_valid_until > clock_timestamp()
            "#,
        )
        .bind(attempt_id)
        .bind(completed_login.account_id)
        .bind(completed_login.identity_id)
        .bind(completed_login.session_id)
        .execute(&mut **tx)
        .await?;

        Ok(result.rows_affected() == 1)
    }

    pub async fn cleanup(&self) -> Result<(), sqlx::Error> {
        sqlx::query(
            r#"
            DELETE
            FROM auth.federated_auth_attempts
            WHERE id IN (SELECT id
            FROM auth.federated_auth_attempts
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
