use super::email_verification_service::{EmailVerificationService, VerificationError};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::{
    repositories::{
        accounts_repository::{AccountsRepository, CreateAccountError},
        registration_attempts_repository::RegistrationAttempt,
    },
    security::password_hasher::{PasswordHasher, PasswordHashingError},
    validators::auth_validator::{validate_register_input, RegisterInputValidationError},
};

#[derive(Clone)]
pub struct AuthService {
    accounts_repository: AccountsRepository,
    password_hasher: Arc<dyn PasswordHasher>,
    hash_slots: Arc<Semaphore>,
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

#[derive(Debug)]
pub enum PasswordCheckError {
    Busy,
    Failed,
}

impl AuthService {
    pub async fn check_password(
        &self,
        password: String,
        expected: Option<String>,
    ) -> Result<bool, PasswordCheckError> {
        let permit = self
            .hash_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| PasswordCheckError::Busy)?;
        let hasher = self.password_hasher.clone();

        tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let password = zeroize::Zeroizing::new(password);
            match expected {
                Some(hash) => hasher.verify_password(&password, &hash),
                None => hasher.hash_password(&password).map(|_| false),
            }
        })
        .await
        .map_err(|_| PasswordCheckError::Failed)?
        .map_err(|_| PasswordCheckError::Failed)
    }

    pub fn new(
        accounts_repository: AccountsRepository,
        password_hasher: Arc<dyn PasswordHasher>,
        verification: EmailVerificationService,
        max_concurrent_hashes: usize,
    ) -> Self {
        Self {
            accounts_repository,
            password_hasher,
            verification,
            hash_slots: Arc::new(Semaphore::new(max_concurrent_hashes)),
        }
    }

    pub async fn register(
        &self,
        email: String,
        password: String,
        budget: &dyn crate::ports::email_verification::RecipientBudget,
    ) -> Result<(RegistrationAttempt, bool), RegisterError> {
        let valid_input =
            validate_register_input(email, password).map_err(RegisterError::InvalidInput)?;

        let permit = self
            .hash_slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| RegisterError::Busy)?;
        let hasher = self.password_hasher.clone();

        let password_hash = tokio::task::spawn_blocking(move || {
            let _permit = permit;
            let password = zeroize::Zeroizing::new(valid_input.password);
            hasher.hash_password(&password)
        })
        .await
        .map_err(RegisterError::Worker)?
        .map_err(RegisterError::PasswordHashing)?;

        if self
            .accounts_repository
            .is_registered(&valid_input.email)
            .await
            .map_err(|error| RegisterError::CreateAccount(CreateAccountError::Database(error)))?
        {
            return Err(RegisterError::CreateAccount(
                CreateAccountError::EmailAlreadyExists,
            ));
        }

        self.verification
            .start(&valid_input.email, &password_hash, budget)
            .await
            .map_err(RegisterError::Verification)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
        repositories::registration_attempts_repository::RegistrationAttemptsRepository,
    };

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
            Arc::new(BlockingHasher {
                started: std::sync::Mutex::new(Some(started_tx)),
                release: std::sync::Mutex::new(release_rx),
            }),
            verification,
            1,
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
        assert_eq!(service.hash_slots.available_permits(), 1);
    }
}
