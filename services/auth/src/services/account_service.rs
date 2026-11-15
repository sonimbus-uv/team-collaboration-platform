use super::{
    email_verification_service::{EmailVerificationService, VerificationError},
    password_processor::{PasswordProcessingError, PasswordProcessor},
};
use crate::{
    repositories::{
        accounts_repository::{AccountsRepository, CreateAccountError},
        registration_attempts_repository::RegistrationAttempt,
    },
    security::password_hasher::PasswordHashingError,
    validators::auth_validator::{validate_register_input, RegisterInputValidationError},
};

#[derive(Clone)]
pub struct AuthService {
    accounts_repository: AccountsRepository,
    password_processor: PasswordProcessor,
    verification: EmailVerificationService,
}

#[derive(Debug)]
pub enum RegisterError {
    InvalidInput(RegisterInputValidationError),
    PasswordHashing(PasswordHashingError),
    CreateAccount(CreateAccountError),
    Verification(VerificationError),
    Busy,
    Worker(tokio::task::JoinError),
}

impl From<PasswordProcessingError> for RegisterError {
    fn from(error: PasswordProcessingError) -> Self {
        match error {
            PasswordProcessingError::Busy => Self::Busy,
            PasswordProcessingError::Hashing(error) => Self::PasswordHashing(error),
            PasswordProcessingError::Worker(error) => Self::Worker(error),
        }
    }
}

impl AuthService {
    pub fn new(
        accounts_repository: AccountsRepository,
        password_processor: PasswordProcessor,
        verification: EmailVerificationService,
    ) -> Self {
        Self {
            accounts_repository,
            password_processor,
            verification,
        }
    }

    pub(crate) fn password_processor(&self) -> PasswordProcessor {
        self.password_processor.clone()
    }

    pub async fn register(
        &self,
        email: String,
        password: String,
        budget: &dyn crate::ports::email_verification::RecipientBudget,
    ) -> Result<(RegistrationAttempt, bool), RegisterError> {
        let registration_input =
            validate_register_input(email, password).map_err(RegisterError::InvalidInput)?;

        let password_hash = self
            .password_processor
            .hash(registration_input.password)
            .await
            .map_err(RegisterError::from)?;

        self.ensure_email_available(&registration_input.email)
            .await?;

        self.verification
            .start(&registration_input.email, &password_hash, budget)
            .await
            .map_err(RegisterError::Verification)
    }

    async fn ensure_email_available(&self, email: &str) -> Result<(), RegisterError> {
        if self
            .accounts_repository
            .is_registered(email)
            .await
            .map_err(|error| RegisterError::CreateAccount(CreateAccountError::Database(error)))?
        {
            return Err(RegisterError::CreateAccount(
                CreateAccountError::EmailAlreadyExists,
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::security::password_hasher::PasswordHasher;
    use crate::{
        ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
        repositories::registration_attempts_repository::RegistrationAttemptsRepository,
    };
    use std::sync::Arc;

    struct NoopSender;

    #[async_trait::async_trait]
    impl VerificationEmailSender for NoopSender {
        async fn send(&self, _: &str, _: &str, _: &str) -> Result<(), EmailDeliveryError> {
            Ok(())
        }
    }

    struct BlockingHasher {
        started: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<std::thread::ThreadId>>>,
        release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
    }

    impl PasswordHasher for BlockingHasher {
        fn hash_password(&self, _: &str) -> Result<String, PasswordHashingError> {
            self.started
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(std::thread::current().id())
                .unwrap();
            let _ = self
                .release
                .lock()
                .unwrap()
                .recv_timeout(std::time::Duration::from_secs(5));
            Err(PasswordHashingError::HashFailed)
        }

        fn verify_password(&self, _: &str, _: &str) -> Result<bool, PasswordHashingError> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn hashing_runs_off_runtime_and_rejects_excess_concurrency() {
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://test:test@localhost/test")
            .unwrap();
        let verification = EmailVerificationService::new(
            RegistrationAttemptsRepository::new(pool.clone()),
            Arc::new(NoopSender),
            &[0x17; 32],
        )
        .unwrap();

        let (started_tx, started_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();

        let service = AuthService::new(
            AccountsRepository::new(pool),
            crate::services::password_processor::PasswordProcessor::new(
                Arc::new(BlockingHasher {
                    started: std::sync::Mutex::new(Some(started_tx)),
                    release: std::sync::Mutex::new(release_rx),
                }),
                1,
            ),
            verification,
        );

        let copy = service.clone();
        let first = tokio::spawn(async move {
            copy.register(
                "a@b.com".into(),
                "ValidPass123!".into(),
                &crate::ports::email_verification::TestBudget,
            )
            .await
        });
        let worker_thread = tokio::time::timeout(std::time::Duration::from_secs(5), started_rx)
            .await
            .unwrap()
            .unwrap();

        assert_ne!(worker_thread, std::thread::current().id());
        assert!(matches!(
            service
                .register(
                    "b@b.com".into(),
                    "ValidPass123!".into(),
                    &crate::ports::email_verification::TestBudget
                )
                .await,
            Err(RegisterError::Busy)
        ));
        release_tx.send(()).unwrap();
        assert!(matches!(
            first.await.unwrap(),
            Err(RegisterError::PasswordHashing(_))
        ));
    }
}
