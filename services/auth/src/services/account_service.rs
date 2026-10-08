use super::email_verification_service::{new_token, EmailVerificationService};
use std::sync::Arc;
use tokio::sync::Semaphore;

use crate::{
    repositories::accounts_repository::{Account, AccountsRepository, CreateAccountError},
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
    Busy,
    Worker(tokio::task::JoinError),
}

impl AuthService {
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
    ) -> Result<(Account, bool), RegisterError> {
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
            hasher.hash_password(&valid_input.password)
        })
        .await
        .map_err(RegisterError::Worker)?
        .map_err(RegisterError::PasswordHashing)?;

        let (token, token_hash) = new_token();

        let account = self
            .accounts_repository
            .create_with_password(&valid_input.email, &password_hash, &token_hash)
            .await
            .map_err(RegisterError::CreateAccount)?;
        let sent = match self.verification.send(&account.email, &token).await {
            Ok(()) => true,
            Err(error) => {
                tracing::error!(error = %error, account_id = %account.id, "verification email delivery failed");
                false
            }
        };
        Ok((account, sent))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
        repositories::one_time_tokens_repository::OneTimeTokensRepository,
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
            OneTimeTokensRepository::new(pool.clone()),
            Arc::new(NoopSender),
        );
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
            copy.register("a@b.com".into(), "ValidPass123!".into())
                .await
        });
        let worker_thread = tokio::time::timeout(std::time::Duration::from_secs(5), started_rx)
            .await
            .unwrap()
            .unwrap();
        assert_ne!(worker_thread, std::thread::current().id());
        assert!(matches!(
            service
                .register("b@b.com".into(), "ValidPass123!".into())
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
