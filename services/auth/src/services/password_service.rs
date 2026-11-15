use super::password_processor::PasswordProcessor;
use crate::{
    dto::passwords::{ChangePassword, RecoveryAccepted, ResetPassword},
    ports::email_verification::VerificationEmailSender,
    protection::{auth_rate_limiter::AuthRateLimiter, rate_limit::RateLimitDecision},
    repositories::{
        passwords_repository,
        sessions_repository::{PasswordSnapshot, SessionsRepository},
    },
    security::{recovery_code::RecoveryCodeKey, verification_code::VerificationCodeKey},
    validators::auth_validator::{is_valid_email_format, validate_register_input},
};
use sqlx::PgPool;
use std::{sync::Arc, time::Duration};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};
use uuid::Uuid;
use zeroize::Zeroizing;

#[derive(Debug)]
pub enum PasswordError {
    Malformed,
    InvalidPassword,
    InvalidCurrent,
    InvalidAccess,
    InvalidCode,
    NotAllowed,
    Unchanged,
    Changed,
    Limited(u64),
    Unavailable,
    Internal,
    CommitUncertain,
}

impl From<sqlx::Error> for PasswordError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::PoolTimedOut
            | sqlx::Error::PoolClosed
            | sqlx::Error::Io(_)
            | sqlx::Error::Tls(_) => Self::Unavailable,
            _ => Self::Internal,
        }
    }
}

pub struct PasswordServiceDependencies {
    pub pool: PgPool,
    pub passwords: PasswordProcessor,
    pub sender: Arc<dyn VerificationEmailSender>,
    pub limiter: Arc<AuthRateLimiter>,
}

struct RecoveryCandidate<'a> {
    email: &'a str,
    code: &'a str,
    attempt_id: Uuid,
}

struct RecoveryEmailJob {
    email: String,
    code: Zeroizing<String>,
    attempt_id: Uuid,
    should_send: bool,
}

#[derive(Clone)]
pub struct PasswordService {
    pool: PgPool,
    passwords: PasswordProcessor,
    key: RecoveryCodeKey,
    sender: Arc<dyn VerificationEmailSender>,
    limiter: Arc<AuthRateLimiter>,
    mail_slots: Arc<Semaphore>,
}

impl PasswordService {
    pub fn new(dependencies: PasswordServiceDependencies, secret: &[u8]) -> anyhow::Result<Self> {
        Ok(Self {
            pool: dependencies.pool,
            passwords: dependencies.passwords,
            key: RecoveryCodeKey::new(secret)?,
            sender: dependencies.sender,
            limiter: dependencies.limiter,
            mail_slots: Arc::new(Semaphore::new(32)),
        })
    }

    pub async fn change(
        &self,
        account_id: Uuid,
        session_id: Uuid,
        request: ChangePassword,
    ) -> Result<(), PasswordError> {
        let ChangePassword {
            current_password,
            new_password,
        } = request;

        if current_password.chars().count() > 128 {
            return Err(PasswordError::Malformed);
        }

        let mut tx = self.pool.begin().await?;
        Self::validate_change_access(&mut tx, account_id, session_id).await?;
        tx.rollback().await?;

        match self
            .limiter
            .check_change_account(account_id)
            .await
            .map_err(|_| PasswordError::Unavailable)?
        {
            RateLimitDecision::Allowed => {}
            RateLimitDecision::Limited { retry_after_secs } => {
                return Err(PasswordError::Limited(retry_after_secs))
            }
        }

        let password_snapshot = passwords_repository::password(&self.pool, account_id)
            .await?
            .ok_or(PasswordError::NotAllowed)?;
        if !self
            .passwords
            .verify(
                current_password,
                Some(password_snapshot.password_hash.clone()),
            )
            .await
            .map_err(|_| PasswordError::Unavailable)?
        {
            return Err(PasswordError::InvalidCurrent);
        }

        let password_hash = self
            .prepare_new_password(new_password, &password_snapshot)
            .await?;

        let mut tx = self.pool.begin().await?;
        Self::validate_change_access(&mut tx, account_id, session_id).await?;
        if !SessionsRepository::password_unchanged(&mut tx, &password_snapshot).await? {
            return Err(PasswordError::Changed);
        }

        passwords_repository::install(&mut tx, &password_snapshot, &password_hash).await?;

        tx.commit()
            .await
            .map_err(|_| PasswordError::CommitUncertain)
    }

    pub async fn forgot(&self, raw_email: String) -> Result<RecoveryAccepted, PasswordError> {
        let email = raw_email.trim().to_lowercase();
        if !is_valid_email_format(&email) {
            return Err(PasswordError::Malformed);
        }
        // Reserve for every syntactically valid address, not just eligible accounts.
        let mail_permit = self
            .mail_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| PasswordError::Unavailable)?;
        let recipient_allowed = matches!(
            self.limiter
                .check_recipient(&email)
                .await
                .map_err(|_| PasswordError::Unavailable)?,
            RateLimitDecision::Allowed
        );
        let recovery_attempt_id = Uuid::new_v4();
        let code = VerificationCodeKey::generate();

        let mut tx = self.pool.begin().await?;
        let recovery_created = self
            .create_recovery_if_allowed(
                &mut tx,
                RecoveryCandidate {
                    email: &email,
                    code: &code,
                    attempt_id: recovery_attempt_id,
                },
                recipient_allowed,
            )
            .await?;

        let expires_at: String = if recovery_created {
            passwords_repository::recovery_expiry(&mut tx, recovery_attempt_id).await?
        } else {
            passwords_repository::synthetic_recovery_expiry(&mut tx).await?
        };

        tx.commit()
            .await
            .map_err(|_| PasswordError::CommitUncertain)?;

        self.schedule_recovery_email(
            RecoveryEmailJob {
                email,
                code,
                attempt_id: recovery_attempt_id,
                should_send: recovery_created,
            },
            mail_permit,
        );

        Ok(RecoveryAccepted {
            recovery_attempt_id,
            expires_at,
            message:
                "Si la cuenta permite recuperar la contraseña, recibirás un código por correo.",
        })
    }

    pub async fn reset(&self, request: ResetPassword) -> Result<(), PasswordError> {
        let initial_recovery_attempt =
            passwords_repository::recovery(&self.pool, request.recovery_attempt_id)
                .await?
                .ok_or(PasswordError::InvalidCode)?;

        let password_snapshot =
            passwords_repository::password(&self.pool, initial_recovery_attempt.account_id)
                .await?
                .ok_or(PasswordError::InvalidCode)?;

        let mut tx = self.pool.begin().await?;
        let recovery_attempt =
            Self::validate_recovery(&mut tx, initial_recovery_attempt.id, &password_snapshot)
                .await?;
        if !self.key.verify(
            recovery_attempt.id,
            recovery_attempt.account_id,
            recovery_attempt.credential_version,
            &request.code,
            &recovery_attempt.code_mac,
        ) {
            passwords_repository::record_failed_recovery(&mut tx, recovery_attempt.id).await?;

            tx.commit()
                .await
                .map_err(|_| PasswordError::CommitUncertain)?;
            return Err(PasswordError::InvalidCode);
        }
        tx.rollback().await?;

        let password_hash = self
            .prepare_new_password(request.new_password, &password_snapshot)
            .await?;

        let mut tx = self.pool.begin().await?;
        let recovery_attempt =
            Self::validate_recovery(&mut tx, recovery_attempt.id, &password_snapshot).await?;
        passwords_repository::consume_recovery(&mut tx, recovery_attempt.id).await?;

        passwords_repository::install(&mut tx, &password_snapshot, &password_hash).await?;

        tx.commit()
            .await
            .map_err(|_| PasswordError::CommitUncertain)
    }

    pub async fn cleanup(&self) -> Result<u64, sqlx::Error> {
        passwords_repository::cleanup(&self.pool).await
    }

    async fn validate_change_access(
        tx: &mut passwords_repository::Tx<'_>,
        account_id: Uuid,
        session_id: Uuid,
    ) -> Result<(), PasswordError> {
        if SessionsRepository::lock_password_account(tx, account_id).await? != Some(true)
            || !passwords_repository::session_active(tx, account_id, session_id).await?
        {
            return Err(PasswordError::InvalidAccess);
        }

        Ok(())
    }

    async fn prepare_new_password(
        &self,
        new_password: String,
        password_snapshot: &PasswordSnapshot,
    ) -> Result<String, PasswordError> {
        validate_new(&new_password)?;
        if self
            .passwords
            .verify(
                new_password.clone(),
                Some(password_snapshot.password_hash.clone()),
            )
            .await
            .map_err(|_| PasswordError::Unavailable)?
        {
            return Err(PasswordError::Unchanged);
        }

        self.passwords
            .hash(new_password)
            .await
            .map_err(|_| PasswordError::Unavailable)
    }

    // Recheck under locks after hashing: another request may have changed the credentials.
    async fn validate_recovery(
        tx: &mut passwords_repository::Tx<'_>,
        attempt_id: Uuid,
        password_snapshot: &PasswordSnapshot,
    ) -> Result<passwords_repository::RecoveryRow, PasswordError> {
        if SessionsRepository::lock_password_account(tx, password_snapshot.account_id).await?
            != Some(true)
        {
            return Err(PasswordError::InvalidCode);
        }
        if !SessionsRepository::password_unchanged(tx, password_snapshot).await? {
            return Err(PasswordError::Changed);
        }
        let recovery_attempt = passwords_repository::lock_recovery(tx, attempt_id)
            .await?
            .ok_or(PasswordError::InvalidCode)?;
        if !recovery_attempt.usable
            || recovery_attempt.credential_version != password_snapshot.version
        {
            return Err(PasswordError::InvalidCode);
        }

        Ok(recovery_attempt)
    }

    async fn create_recovery_if_allowed(
        &self,
        tx: &mut passwords_repository::Tx<'_>,
        candidate: RecoveryCandidate<'_>,
        recipient_allowed: bool,
    ) -> Result<bool, PasswordError> {
        let RecoveryCandidate {
            email,
            code,
            attempt_id,
        } = candidate;
        let Some(account_id) = passwords_repository::lock_account_by_email(tx, email).await? else {
            return Ok(false);
        };

        let account_eligible = passwords_repository::recovery_eligible(tx, account_id).await?;
        let cooldown_elapsed =
            passwords_repository::recovery_cooldown_elapsed(tx, account_id).await?;
        if !account_eligible || !recipient_allowed || !cooldown_elapsed {
            return Ok(false);
        }

        let credential_version =
            passwords_repository::lock_credential_version(tx, account_id).await?;
        passwords_repository::invalidate(tx, account_id).await?;
        let code_mac = self
            .key
            .digest(attempt_id, account_id, credential_version, code);
        passwords_repository::create_recovery(
            tx,
            attempt_id,
            account_id,
            credential_version,
            &code_mac,
        )
        .await?;

        Ok(true)
    }

    fn schedule_recovery_email(&self, job: RecoveryEmailJob, mail_permit: OwnedSemaphorePermit) {
        // Bounded, non-durable delivery keeps provider latency out of the HTTP response.
        // Both real and synthetic requests occupy their admission slot for the same period.
        let RecoveryEmailJob {
            email,
            code,
            attempt_id,
            should_send,
        } = job;
        let service = self.clone();
        tokio::spawn(async move {
            let _mail_permit = mail_permit;
            let admission_deadline = tokio::time::Instant::now() + Duration::from_secs(11);
            if should_send {
                let _ = tokio::time::timeout(Duration::from_secs(10), async {
                    if passwords_repository::recovery(&service.pool, attempt_id)
                        .await
                        .ok()
                        .flatten()
                        .is_some_and(|recovery_attempt| recovery_attempt.usable)
                    {
                        let _ = service.sender.send_recovery(&email, &code, "es-MX").await;
                    }
                })
                .await;
            }
            tokio::time::sleep_until(admission_deadline).await;
        });
    }
}

fn validate_new(password: &str) -> Result<(), PasswordError> {
    validate_register_input("policy@example.invalid".into(), password.into())
        .map(|_| ())
        .map_err(|_| PasswordError::InvalidPassword)
}
