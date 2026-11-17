use crate::{
    ports::email_verification::{RecipientBudget, VerificationEmailSender},
    repositories::{
        accounts_repository::Account,
        registration_attempts_repository::{RegistrationAttempt, RegistrationAttemptsRepository},
    },
    security::registration_code::RegistrationCodeKey,
};
use std::sync::Arc;
use uuid::Uuid;

pub use crate::repositories::registration_attempts_repository::RegistrationError as VerificationError;

#[derive(Clone)]
pub struct EmailVerificationService {
    repository: RegistrationAttemptsRepository,
    sender: Arc<dyn VerificationEmailSender>,
    key: RegistrationCodeKey,
}

impl EmailVerificationService {
    pub fn new(
        repository: RegistrationAttemptsRepository,
        sender: Arc<dyn VerificationEmailSender>,
        secret: &[u8],
    ) -> anyhow::Result<Self> {
        Ok(Self {
            repository,
            sender,
            key: RegistrationCodeKey::new(secret)?,
        })
    }

    async fn consume_recipient_budget(
        email: &str,
        recipient_budget: &dyn RecipientBudget,
    ) -> Result<(), VerificationError> {
        match recipient_budget.consume(email).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(VerificationError::Limited(600)),
            Err(()) => Err(VerificationError::Unavailable),
        }
    }

    async fn deliver_attempt(
        &self,
        attempt: RegistrationAttempt,
        code: &str,
    ) -> (RegistrationAttempt, bool) {
        // Delivery runs after the repository transaction has completed.
        let email_sent = self
            .sender
            .send(&attempt.email, code, "es-MX")
            .await
            .is_ok();
        (attempt, email_sent)
    }

    pub async fn start(
        &self,
        email: &str,
        password_hash: &str,
        recipient_budget: &dyn RecipientBudget,
    ) -> Result<(RegistrationAttempt, bool), VerificationError> {
        Self::consume_recipient_budget(email, recipient_budget).await?;
        let registration_attempt_id = Uuid::new_v4();
        let code = RegistrationCodeKey::generate();
        let code_mac = self.key.digest(registration_attempt_id, 0, &code);
        let attempt = self
            .repository
            .create(email, password_hash, registration_attempt_id, &code_mac)
            .await?;
        Ok(self.deliver_attempt(attempt, &code).await)
    }

    pub async fn confirm(
        &self,
        registration_attempt_id: Uuid,
        code: &str,
    ) -> Result<Account, VerificationError> {
        self.repository
            .confirm(registration_attempt_id, code, &self.key)
            .await
    }

    pub async fn resend(
        &self,
        registration_attempt_id: Uuid,
        recipient_budget: &dyn RecipientBudget,
    ) -> Result<(RegistrationAttempt, bool), VerificationError> {
        let attempt = self
            .repository
            .resend_candidate(registration_attempt_id)
            .await?;
        Self::consume_recipient_budget(&attempt.email, recipient_budget).await?;
        let mut code = RegistrationCodeKey::generate();
        let attempt = self
            .repository
            .reissue(registration_attempt_id, &self.key, &mut code)
            .await?;
        Ok(self.deliver_attempt(attempt, &code).await)
    }
}
