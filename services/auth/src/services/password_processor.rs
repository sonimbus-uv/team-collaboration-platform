use crate::security::password_hasher::{PasswordHasher, PasswordHashingError};
use std::sync::Arc;
use tokio::sync::Semaphore;
use zeroize::Zeroizing;

pub enum PasswordProcessingError {
    Busy,
    Hashing(PasswordHashingError),
    Worker(tokio::task::JoinError),
}

impl std::fmt::Debug for PasswordProcessingError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Busy => formatter.write_str("Busy"),
            Self::Hashing(error) => formatter.debug_tuple("Hashing").field(error).finish(),
            Self::Worker(_) => formatter.write_str("Worker"),
        }
    }
}

#[derive(Clone)]
pub struct PasswordProcessor {
    hasher: Arc<dyn PasswordHasher>,
    slots: Arc<Semaphore>,
}

impl PasswordProcessor {
    pub fn new(hasher: Arc<dyn PasswordHasher>, max_concurrent_operations: usize) -> Self {
        Self {
            hasher,
            slots: Arc::new(Semaphore::new(max_concurrent_operations)),
        }
    }

    pub async fn hash(&self, password: String) -> Result<String, PasswordProcessingError> {
        self.run(password, |hasher, password| hasher.hash_password(password))
            .await
    }

    pub async fn verify(
        &self,
        password: String,
        expected_hash: Option<String>,
    ) -> Result<bool, PasswordProcessingError> {
        self.run(password, move |hasher, password| match expected_hash {
            Some(hash) => hasher.verify_password(password, &hash),
            // Missing accounts still perform password work to reduce timing differences.
            None => hasher.hash_password(password).map(|_| false),
        })
        .await
    }

    async fn run<T, F>(&self, password: String, operation: F) -> Result<T, PasswordProcessingError>
    where
        T: Send + 'static,
        F: FnOnce(&dyn PasswordHasher, &str) -> Result<T, PasswordHashingError> + Send + 'static,
    {
        let password = Zeroizing::new(password);
        let permit = self
            .slots
            .clone()
            .try_acquire_owned()
            .map_err(|_| PasswordProcessingError::Busy)?;
        let hasher = self.hasher.clone();

        tokio::task::spawn_blocking(move || {
            // The worker keeps its permit even if the awaiting request is cancelled.
            let _permit = permit;
            operation(hasher.as_ref(), &password)
        })
        .await
        .map_err(PasswordProcessingError::Worker)?
        .map_err(PasswordProcessingError::Hashing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::{
            atomic::{AtomicUsize, Ordering},
            mpsc, Mutex,
        },
        time::Duration,
    };
    use tokio::sync::oneshot;

    struct GatedHasher {
        entered: Mutex<Option<oneshot::Sender<std::thread::ThreadId>>>,
        release: Mutex<mpsc::Receiver<()>>,
    }

    impl PasswordHasher for GatedHasher {
        fn hash_password(&self, _: &str) -> Result<String, PasswordHashingError> {
            self.entered
                .lock()
                .unwrap()
                .take()
                .unwrap()
                .send(std::thread::current().id())
                .unwrap();
            self.release
                .lock()
                .unwrap()
                .recv_timeout(Duration::from_secs(5))
                .unwrap();
            Ok("test-hash".into())
        }

        fn verify_password(&self, _: &str, _: &str) -> Result<bool, PasswordHashingError> {
            panic!("Verification must not run while the shared capacity is exhausted");
        }
    }

    fn gated_processor() -> (
        PasswordProcessor,
        oneshot::Receiver<std::thread::ThreadId>,
        mpsc::Sender<()>,
    ) {
        let (entered_tx, entered_rx) = oneshot::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let processor = PasswordProcessor::new(
            Arc::new(GatedHasher {
                entered: Mutex::new(Some(entered_tx)),
                release: Mutex::new(release_rx),
            }),
            1,
        );
        (processor, entered_rx, release_tx)
    }

    #[tokio::test]
    async fn clones_share_capacity_and_hashing_runs_off_runtime() {
        let (processor, entered, release) = gated_processor();
        let worker_processor = processor.clone();
        let worker = tokio::spawn(async move { worker_processor.hash("test".into()).await });
        let worker_thread = tokio::time::timeout(Duration::from_secs(5), entered)
            .await
            .unwrap()
            .unwrap();

        assert_ne!(worker_thread, std::thread::current().id());
        assert!(matches!(
            processor
                .verify("test".into(), Some("test-hash".into()))
                .await,
            Err(PasswordProcessingError::Busy)
        ));
        release.send(()).unwrap();
        assert_eq!(worker.await.unwrap().unwrap(), "test-hash");
        assert_eq!(processor.slots.available_permits(), 1);
    }

    #[tokio::test]
    async fn cancellation_keeps_capacity_reserved_until_the_worker_finishes() {
        let (processor, entered, release) = gated_processor();
        let worker_processor = processor.clone();
        let request = tokio::spawn(async move { worker_processor.hash("test".into()).await });
        tokio::time::timeout(Duration::from_secs(5), entered)
            .await
            .unwrap()
            .unwrap();

        request.abort();
        assert!(request.await.unwrap_err().is_cancelled());
        assert!(matches!(
            processor.hash("other".into()).await,
            Err(PasswordProcessingError::Busy)
        ));
        release.send(()).unwrap();
        tokio::time::timeout(Duration::from_secs(5), async {
            while processor.slots.available_permits() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .unwrap();
    }

    struct CountingHasher {
        hash_calls: AtomicUsize,
        verify_calls: AtomicUsize,
    }

    impl PasswordHasher for CountingHasher {
        fn hash_password(&self, _: &str) -> Result<String, PasswordHashingError> {
            self.hash_calls.fetch_add(1, Ordering::SeqCst);
            Ok("test-hash".into())
        }

        fn verify_password(&self, password: &str, _: &str) -> Result<bool, PasswordHashingError> {
            self.verify_calls.fetch_add(1, Ordering::SeqCst);
            Ok(password == "correct")
        }
    }

    #[tokio::test]
    async fn missing_accounts_still_hash_and_wrong_passwords_are_not_errors() {
        let hasher = Arc::new(CountingHasher {
            hash_calls: AtomicUsize::new(0),
            verify_calls: AtomicUsize::new(0),
        });
        let processor = PasswordProcessor::new(hasher.clone(), 1);

        assert!(!processor.verify("test".into(), None).await.unwrap());
        assert_eq!(hasher.hash_calls.load(Ordering::SeqCst), 1);
        assert_eq!(hasher.verify_calls.load(Ordering::SeqCst), 0);
        assert!(!processor
            .verify("wrong".into(), Some("test-hash".into()))
            .await
            .unwrap());
        assert!(processor
            .verify("correct".into(), Some("test-hash".into()))
            .await
            .unwrap());
    }

    struct FailingHasher;

    impl PasswordHasher for FailingHasher {
        fn hash_password(&self, password: &str) -> Result<String, PasswordHashingError> {
            if password == "panic" {
                panic!("Test worker failure");
            }
            Err(PasswordHashingError::HashFailed)
        }

        fn verify_password(&self, _: &str, _: &str) -> Result<bool, PasswordHashingError> {
            Err(PasswordHashingError::InvalidHash)
        }
    }

    #[tokio::test]
    async fn processing_failures_and_worker_panics_release_capacity() {
        let processor = PasswordProcessor::new(Arc::new(FailingHasher), 1);
        assert!(matches!(
            processor.hash("test".into()).await,
            Err(PasswordProcessingError::Hashing(
                PasswordHashingError::HashFailed
            ))
        ));
        assert_eq!(processor.slots.available_permits(), 1);
        assert!(matches!(
            processor
                .verify("test".into(), Some("invalid".into()))
                .await,
            Err(PasswordProcessingError::Hashing(
                PasswordHashingError::InvalidHash
            ))
        ));
        assert_eq!(processor.slots.available_permits(), 1);
        assert!(matches!(
            processor.hash("panic".into()).await,
            Err(PasswordProcessingError::Worker(_))
        ));
        assert_eq!(processor.slots.available_permits(), 1);
    }
}
