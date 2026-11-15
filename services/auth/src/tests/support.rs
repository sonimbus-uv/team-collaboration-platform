use std::path::{Path, PathBuf};
use std::time::Duration;
use tokio::task::{AbortHandle, JoinHandle};

pub(crate) struct TemporaryDirectory {
    path: PathBuf,
}

impl TemporaryDirectory {
    pub(crate) fn new(prefix: &str) -> Self {
        let path = std::env::temp_dir().join(format!("{prefix}{}", uuid::Uuid::new_v4()));
        std::fs::create_dir(&path).expect("Cannot create disposable test directory");
        Self { path }
    }

    pub(crate) fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TemporaryDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

pub(crate) struct AbortOnDrop(AbortHandle);

impl AbortOnDrop {
    pub(crate) fn new<T>(task: &JoinHandle<T>) -> Self {
        Self(task.abort_handle())
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        self.0.abort();
    }
}

pub(crate) async fn mailpit_message(recipient_email: &str) -> serde_json::Value {
    let api_url = std::env::var("AUTH_TEST_MAILPIT_API_URL")
        .expect("Mailpit tests require AUTH_TEST_MAILPIT_API_URL");
    let client = reqwest::Client::builder()
        .no_proxy()
        .timeout(Duration::from_secs(3))
        .build()
        .expect("Cannot create Mailpit test client");

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let search: serde_json::Value = client
                .get(format!("{api_url}/api/v1/search"))
                .query(&[("query", format!("to:{recipient_email}"))])
                .send()
                .await
                .unwrap()
                .error_for_status()
                .unwrap()
                .json()
                .await
                .unwrap();
            let messages = search["messages"]
                .as_array()
                .expect("Invalid Mailpit search response");
            if let Some(message) = messages.first() {
                let message_id = message["ID"].as_str().expect("Missing Mailpit message ID");
                return client
                    .get(format!("{api_url}/api/v1/message/{message_id}"))
                    .send()
                    .await
                    .unwrap()
                    .error_for_status()
                    .unwrap()
                    .json()
                    .await
                    .unwrap();
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
    })
    .await
    .expect("Mailpit message was not delivered before the test deadline")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn temporary_directory_is_removed_when_a_test_panics() {
        let mut directory_path = None;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let directory = TemporaryDirectory::new("sonimbus-cleanup-test-");
            directory_path = Some(directory.path().to_path_buf());
            std::fs::write(directory.path().join("test-only.txt"), "fixture").unwrap();
            panic!("Simulated test failure");
        }));
        assert!(result.is_err());
        assert!(!directory_path.unwrap().exists());
    }

    #[tokio::test]
    async fn task_guard_aborts_background_work_when_its_owner_panics() {
        let worker = tokio::spawn(std::future::pending::<()>());
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = AbortOnDrop::new(&worker);
            panic!("Simulated test failure");
        }));
        assert!(result.is_err());
        let error = tokio::time::timeout(Duration::from_secs(1), worker)
            .await
            .unwrap()
            .unwrap_err();
        assert!(error.is_cancelled());
    }
}
