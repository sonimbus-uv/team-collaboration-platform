use std::{sync::Arc, time::Duration};

use tokio::sync::Semaphore;
use uuid::Uuid;

use crate::{
    dto::contact_email::{ContactEmail, ContactVerificationAccepted},
    ports::email_verification::VerificationEmailSender,
    protection::{
        auth_rate_limiter::AuthRateLimiter,
        rate_limit::{ContactAccountOperation, RateLimitDecision},
    },
    repositories::{
        contact_email_repository::{ContactEmailRepository, NewContactAttempt},
        session_authentication_repository::SessionActor,
    },
    security::{
        contact_email_code::{ContactCodeContext, ContactEmailCodeKey},
        verification_code::VerificationCodeKey,
    },
    validators::auth_validator::is_valid_email_format,
};

const MAX_CONCURRENT_EMAILS: usize = 16;
const EMAIL_DELIVERY_TIMEOUT: Duration = Duration::from_secs(12);
const MAX_CODE_REPLACEMENTS: i32 = 3;
const REPLACEMENT_LIMIT_RETRY_SECONDS: u64 = 600;

pub enum ContactEmailError {
    Malformed,
    InvalidAccess,
    InvalidCode,
    InUse,
    ChangeNotSupported,
    Limited(u64),
    Unavailable,
    CommitUncertain,
    Database,
}

impl From<sqlx::Error> for ContactEmailError {
    fn from(error: sqlx::Error) -> Self {
        match error {
            sqlx::Error::Database(ref database)
                if database.code().as_deref() == Some("23505")
                    && database.constraint() == Some("accounts_email_unique") =>
            {
                Self::InUse
            }
            sqlx::Error::PoolTimedOut
            | sqlx::Error::PoolClosed
            | sqlx::Error::Io(_)
            | sqlx::Error::Tls(_) => Self::Unavailable,
            _ => Self::Database,
        }
    }
}

pub enum ContactRequestResult {
    Accepted(ContactVerificationAccepted),
    AlreadyVerified,
}

#[derive(Clone)]
pub struct ContactEmailService {
    repository: ContactEmailRepository,
    sender: Arc<dyn VerificationEmailSender>,
    limiter: Arc<AuthRateLimiter>,
    key: ContactEmailCodeKey,
    mail_slots: Arc<Semaphore>,
}

impl ContactEmailService {
    pub fn new(
        repository: ContactEmailRepository,
        sender: Arc<dyn VerificationEmailSender>,
        limiter: Arc<AuthRateLimiter>,
        secret: &[u8],
    ) -> anyhow::Result<Self> {
        Ok(Self {
            repository,
            sender,
            limiter,
            key: ContactEmailCodeKey::new(secret)?,
            mail_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_EMAILS)),
        })
    }

    pub async fn snapshot(&self, actor: SessionActor) -> Result<ContactEmail, ContactEmailError> {
        self.repository
            .snapshot(actor)
            .await?
            .ok_or(ContactEmailError::InvalidAccess)
    }

    async fn check_send_budget(
        &self,
        actor: SessionActor,
        email: &str,
    ) -> Result<(), ContactEmailError> {
        check_budget(
            self.limiter
                .check_contact_account(actor.account_id, ContactAccountOperation::Send)
                .await,
        )?;
        check_budget(self.limiter.check_recipient(email).await)
    }

    pub async fn start(
        &self,
        actor: SessionActor,
        email: String,
    ) -> Result<ContactRequestResult, ContactEmailError> {
        let email = email.trim().to_lowercase();
        if !is_valid_email_format(&email) {
            return Err(ContactEmailError::Malformed);
        }
        // Admission reads avoid charging no-op requests; writes revalidate under lock.
        let preflight = self
            .repository
            .preflight(actor, &email)
            .await?
            .ok_or(ContactEmailError::InvalidAccess)?;
        if let Some(contact) = preflight.email {
            return existing_contact_result(&contact, &email);
        }
        if preflight.in_use {
            return Err(ContactEmailError::InUse);
        }
        if let (Some(attempt_id), Some(expires_at)) = (preflight.live_id, preflight.live_expires) {
            return Ok(ContactRequestResult::Accepted(
                ContactVerificationAccepted {
                    verification_attempt_id: attempt_id,
                    expires_at,
                },
            ));
        }

        let _slot = self
            .mail_slots
            .try_acquire()
            .map_err(|_| ContactEmailError::Unavailable)?;
        self.check_send_budget(actor, &email).await?;
        let attempt_id = Uuid::new_v4();
        let code = VerificationCodeKey::generate();
        let mut tx = self.repository.pool.begin().await?;
        let owner = ContactEmailRepository::lock_owner(&mut tx, actor)
            .await?
            .ok_or(ContactEmailError::InvalidAccess)?;
        if let Some(contact) = owner.email {
            return existing_contact_result(&contact, &email);
        }
        if ContactEmailRepository::in_use(&mut tx, actor.account_id, &email).await? {
            return Err(ContactEmailError::InUse);
        }

        let version = ContactEmailRepository::propose(&mut tx, actor.account_id, &email).await?;
        if let Some(attempt) = ContactEmailRepository::lock_attempt(&mut tx, actor, None).await? {
            if attempt.usable {
                let accepted = attempt.accepted();
                tx.commit()
                    .await
                    .map_err(|_| ContactEmailError::CommitUncertain)?;
                return Ok(ContactRequestResult::Accepted(accepted));
            }
        }
        ContactEmailRepository::invalidate(&mut tx, actor.account_id).await?;
        let context = ContactCodeContext {
            account_id: actor.account_id,
            email: &email,
            attempt_id,
            contact_version: version,
            code_version: 0,
        };

        let code_mac = self.key.digest(&context, &code);
        let accepted = ContactEmailRepository::create(
            &mut tx,
            NewContactAttempt {
                context,
                code_mac: &code_mac,
            },
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| ContactEmailError::CommitUncertain)?;
        self.send_verification_code(&email, &code).await;
        Ok(ContactRequestResult::Accepted(accepted))
    }

    pub async fn resend(
        &self,
        actor: SessionActor,
        attempt_id: Uuid,
    ) -> Result<ContactVerificationAccepted, ContactEmailError> {
        let candidate = self
            .repository
            .resend_candidate(actor, attempt_id)
            .await?
            .ok_or(ContactEmailError::InvalidCode)?;
        validate_resend_eligibility(candidate.resend_wait, candidate.code_version)?;
        let email = candidate.email;
        let _slot = self
            .mail_slots
            .try_acquire()
            .map_err(|_| ContactEmailError::Unavailable)?;
        self.check_send_budget(actor, &email).await?;
        let mut code = VerificationCodeKey::generate();
        let mut tx = self.repository.pool.begin().await?;
        ContactEmailRepository::lock_owner(&mut tx, actor)
            .await?
            .ok_or(ContactEmailError::InvalidAccess)?;
        let attempt = ContactEmailRepository::lock_attempt(&mut tx, actor, Some(attempt_id))
            .await?
            .ok_or(ContactEmailError::InvalidCode)?;
        if !attempt.usable {
            return Err(ContactEmailError::InvalidCode);
        }
        validate_resend_eligibility(attempt.resend_wait, attempt.code_version)?;
        while self
            .key
            .verify(&attempt.context(), &code, &attempt.code_mac)
        {
            code = VerificationCodeKey::generate();
        }

        let mut context = attempt.context();
        context.code_version += 1;
        ContactEmailRepository::replace_code(
            &mut tx,
            attempt_id,
            &self.key.digest(&context, &code),
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| ContactEmailError::CommitUncertain)?;
        self.send_verification_code(&email, &code).await;
        Ok(attempt.accepted())
    }

    pub async fn verify(
        &self,
        actor: SessionActor,
        attempt_id: Uuid,
        code: String,
    ) -> Result<(), ContactEmailError> {
        let code = zeroize::Zeroizing::new(code);
        check_budget(
            self.limiter
                .check_contact_account(actor.account_id, ContactAccountOperation::Verify)
                .await,
        )?;
        let email = self
            .repository
            .candidate(actor, attempt_id)
            .await?
            .ok_or(ContactEmailError::InvalidCode)?;
        let mut tx = self.repository.pool.begin().await?;
        ContactEmailRepository::lock_email(&mut tx, &email).await?;
        let owner = ContactEmailRepository::lock_owner(&mut tx, actor)
            .await?
            .ok_or(ContactEmailError::InvalidAccess)?;
        let attempt = ContactEmailRepository::lock_attempt(&mut tx, actor, Some(attempt_id))
            .await?
            .ok_or(ContactEmailError::InvalidCode)?;
        let code_valid = self
            .key
            .verify(&attempt.context(), &code, &attempt.code_mac);
        if attempt.consumed {
            return if owner
                .email
                .as_deref()
                .is_some_and(|contact| contact.eq_ignore_ascii_case(&email))
                && code_valid
            {
                Ok(())
            } else {
                Err(ContactEmailError::InvalidCode)
            };
        }
        if !attempt.usable {
            return Err(ContactEmailError::InvalidCode);
        }
        if owner.email.is_some() {
            return Err(ContactEmailError::ChangeNotSupported);
        }
        if !code_valid {
            ContactEmailRepository::fail(&mut tx, attempt_id).await?;
            tx.commit()
                .await
                .map_err(|_| ContactEmailError::CommitUncertain)?;
            return Err(ContactEmailError::InvalidCode);
        }
        if !ContactEmailRepository::assign(&mut tx, actor, &attempt).await? {
            return Err(ContactEmailError::InvalidCode);
        }
        tx.commit()
            .await
            .map_err(|_| ContactEmailError::CommitUncertain)?;
        Ok(())
    }

    async fn send_verification_code(&self, email: &str, code: &str) {
        // Persistence precedes delivery; a transport failure still allows a later resend.
        let _ = tokio::time::timeout(
            EMAIL_DELIVERY_TIMEOUT,
            self.sender.send_contact_verification(email, code, "es-MX"),
        )
        .await;
    }

    pub async fn cleanup(&self) -> Result<(), ContactEmailError> {
        self.repository.cleanup().await.map_err(Into::into)
    }
}

fn check_budget(
    result: Result<
        RateLimitDecision,
        crate::infrastructure::redis_connection::RedisConnectionError,
    >,
) -> Result<(), ContactEmailError> {
    match result {
        Ok(RateLimitDecision::Allowed) => Ok(()),
        Ok(RateLimitDecision::Limited { retry_after_secs }) => {
            Err(ContactEmailError::Limited(retry_after_secs))
        }
        Err(_) => Err(ContactEmailError::Unavailable),
    }
}

fn existing_contact_result(
    contact: &str,
    requested_email: &str,
) -> Result<ContactRequestResult, ContactEmailError> {
    if contact.eq_ignore_ascii_case(requested_email) {
        Ok(ContactRequestResult::AlreadyVerified)
    } else {
        Err(ContactEmailError::ChangeNotSupported)
    }
}

fn validate_resend_eligibility(
    wait_seconds: i64,
    code_version: i32,
) -> Result<(), ContactEmailError> {
    if wait_seconds > 0 {
        return Err(ContactEmailError::Limited(wait_seconds as u64));
    }
    if code_version >= MAX_CODE_REPLACEMENTS {
        return Err(ContactEmailError::Limited(REPLACEMENT_LIMIT_RETRY_SECONDS));
    }
    Ok(())
}
