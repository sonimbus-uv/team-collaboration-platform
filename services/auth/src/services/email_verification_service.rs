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

    async fn budget(email: &str, budget: &dyn RecipientBudget) -> Result<(), VerificationError> {
        match budget.consume(email).await {
            Ok(true) => Ok(()),
            Ok(false) => Err(VerificationError::Limited(600)),
            Err(()) => Err(VerificationError::Unavailable),
        }
    }

    async fn deliver(&self, email: &str, code: &str) -> bool {
        // No transaction is held during SMTP/HTTP. Errors never include request bodies.
        self.sender.send(email, code, "es-MX").await.is_ok()
    }

    pub async fn start(
        &self,
        email: &str,
        password_hash: &str,
        budget: &dyn RecipientBudget,
    ) -> Result<(RegistrationAttempt, bool), VerificationError> {
        Self::budget(email, budget).await?;
        let id = Uuid::new_v4();
        let code = RegistrationCodeKey::generate();
        let mac = self.key.digest(id, 0, &code);
        let attempt = self
            .repository
            .create(email, password_hash, id, &mac)
            .await?;
        let sent = self.deliver(email, &code).await;
        Ok((attempt, sent))
    }

    pub async fn confirm(&self, id: Uuid, code: &str) -> Result<Account, VerificationError> {
        self.repository.confirm(id, code, &self.key).await
    }

    pub async fn resend(
        &self,
        id: Uuid,
        budget: &dyn RecipientBudget,
    ) -> Result<(RegistrationAttempt, bool), VerificationError> {
        let attempt = self.repository.resend_candidate(id).await?;
        Self::budget(&attempt.email, budget).await?;
        let mut code = RegistrationCodeKey::generate();
        let attempt = self.repository.reissue(id, &self.key, &mut code).await?;
        let sent = self.deliver(&attempt.email, &code).await;
        Ok((attempt, sent))
    }
}
