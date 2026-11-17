use sqlx::PgPool;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use uuid::Uuid;

use crate::{
    ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
    repositories::security_notifications_repository::SecurityNotificationsRepository,
    services::security_notification_service::{NotificationOutcome, SecurityNotificationService},
};

#[derive(Default)]
struct NoticeMail {
    recipients: Mutex<Vec<String>>,
    fail: AtomicBool,
}

#[async_trait::async_trait]
impl VerificationEmailSender for NoticeMail {
    async fn send(&self, _: &str, _: &str, _: &str) -> Result<(), EmailDeliveryError> {
        panic!("Security notices must not send registration codes");
    }

    async fn send_security_notice(
        &self,
        recipient: &str,
        _: &str,
    ) -> Result<(), EmailDeliveryError> {
        self.recipients.lock().unwrap().push(recipient.into());
        if self.fail.load(Ordering::SeqCst) {
            Err(EmailDeliveryError::Rejected(503))
        } else {
            Ok(())
        }
    }
}

async fn enqueue_notification(pool: &PgPool, email: &str) -> (Uuid, Uuid) {
    let account: Uuid = sqlx::query_scalar("INSERT INTO auth.accounts(email, email_verified_at) VALUES ($1, clock_timestamp()) RETURNING id")
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap();
    let identity: Uuid = sqlx::query_scalar("INSERT INTO auth.oauth_identities(account_id, provider, provider_subject) VALUES ($1, 'google', $2) RETURNING id")
        .bind(account)
        .bind(Uuid::new_v4().to_string())
        .fetch_one(pool)
        .await
        .unwrap();
    let event: Uuid = sqlx::query_scalar("INSERT INTO auth.account_security_events(account_id, oauth_identity_id) VALUES ($1, $2) RETURNING id")
        .bind(account)
        .bind(identity)
        .fetch_one(pool)
        .await
        .unwrap();
    let notification: Uuid = sqlx::query_scalar("INSERT INTO auth.security_notification_outbox(event_id, recipient_email) VALUES ($1, $2) RETURNING id")
        .bind(event)
        .bind(email)
        .fetch_one(pool)
        .await
        .unwrap();
    (account, notification)
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_workers_deliver_only_one_claim_and_mark_success(pool: PgPool) {
    let (_, id) = enqueue_notification(&pool, "notice@example.invalid").await;
    let mail = Arc::new(NoticeMail::default());
    let service = SecurityNotificationService::new(
        SecurityNotificationsRepository { pool: pool.clone() },
        mail.clone(),
    );
    let (first, second) = tokio::join!(service.deliver_one(), service.deliver_one());
    let successes = usize::from(matches!(first.unwrap(), NotificationOutcome::Delivered))
        + usize::from(matches!(second.unwrap(), NotificationOutcome::Delivered));
    assert_eq!(successes, 1);
    assert_eq!(mail.recipients.lock().unwrap().len(), 1);
    let row: (i32, bool, Option<String>) = sqlx::query_as("SELECT attempt_count, delivered_at IS NOT NULL, last_error_kind FROM auth.security_notification_outbox WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row, (1, true, None));
}

#[sqlx::test(migrations = "./migrations")]
async fn provider_failures_back_off_and_success_clears_only_the_error_category(pool: PgPool) {
    let (_, id) = enqueue_notification(&pool, "retry@example.invalid").await;
    let mail = Arc::new(NoticeMail::default());
    mail.fail.store(true, Ordering::SeqCst);
    let service = SecurityNotificationService::new(
        SecurityNotificationsRepository { pool: pool.clone() },
        mail.clone(),
    );
    assert!(matches!(
        service.deliver_one().await.unwrap(),
        NotificationOutcome::Retried
    ));
    let row: (i32, String, bool) = sqlx::query_as("SELECT attempt_count, last_error_kind, next_attempt_at > last_attempt_at FROM auth.security_notification_outbox WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(row, (1, "provider_failed".into(), true));
    assert!(matches!(
        service.deliver_one().await.unwrap(),
        NotificationOutcome::Idle
    ));
    sqlx::query("UPDATE auth.security_notification_outbox SET next_attempt_at = clock_timestamp() WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    mail.fail.store(false, Ordering::SeqCst);
    assert!(matches!(
        service.deliver_one().await.unwrap(),
        NotificationOutcome::Delivered
    ));
    let count: i32 = sqlx::query_scalar(
        "SELECT attempt_count FROM auth.security_notification_outbox WHERE id = $1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn expired_worker_claim_cannot_acknowledge_or_reschedule_a_new_generation(pool: PgPool) {
    let (_, id) = enqueue_notification(&pool, "fenced@example.invalid").await;
    let repository = SecurityNotificationsRepository { pool: pool.clone() };
    let first = repository.claim().await.unwrap().unwrap();
    sqlx::query("UPDATE auth.security_notification_outbox SET next_attempt_at = clock_timestamp() WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let second = repository.claim().await.unwrap().unwrap();
    assert!(!repository.delivered(&first).await.unwrap());
    assert!(!repository.retry(&first, "timeout").await.unwrap());
    assert!(repository.delivered(&second).await.unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn delivery_stops_after_ten_attempts_without_deleting_the_event(pool: PgPool) {
    let (_, id) = enqueue_notification(&pool, "exhausted@example.invalid").await;
    sqlx::query("WITH observed AS (SELECT clock_timestamp() AS instant) UPDATE auth.security_notification_outbox SET attempt_count = 9, last_attempt_at = observed.instant, next_attempt_at = observed.instant, last_error_kind = 'provider_failed' FROM observed WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    let mail = Arc::new(NoticeMail::default());
    mail.fail.store(true, Ordering::SeqCst);
    let service = SecurityNotificationService::new(
        SecurityNotificationsRepository { pool: pool.clone() },
        mail.clone(),
    );
    assert!(matches!(
        service.deliver_one().await.unwrap(),
        NotificationOutcome::Exhausted
    ));
    sqlx::query("UPDATE auth.security_notification_outbox SET next_attempt_at = clock_timestamp() WHERE id = $1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        service.deliver_one().await.unwrap(),
        NotificationOutcome::Idle
    ));
    assert_eq!(mail.recipients.lock().unwrap().len(), 1);
    let events: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.account_security_events")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(events, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn worker_never_falls_back_to_provider_email_or_blocks_other_verified_recipients(
    pool: PgPool,
) {
    let (account, _) = enqueue_notification(&pool, "removed@example.invalid").await;
    sqlx::query("UPDATE auth.accounts SET email = NULL, email_verified_at = NULL WHERE id = $1")
        .bind(account)
        .execute(&pool)
        .await
        .unwrap();
    enqueue_notification(&pool, "current@example.invalid").await;
    let mail = Arc::new(NoticeMail::default());
    let service = SecurityNotificationService::new(
        SecurityNotificationsRepository { pool: pool.clone() },
        mail.clone(),
    );
    assert!(matches!(
        service.deliver_one().await.unwrap(),
        NotificationOutcome::Delivered
    ));
    assert_eq!(
        *mail.recipients.lock().unwrap(),
        vec!["current@example.invalid"]
    );
    assert!(matches!(
        service.deliver_one().await.unwrap(),
        NotificationOutcome::Idle
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn mailpit_delivers_informational_google_notice_without_a_code_or_external_resources(
    pool: PgPool,
) {
    let email = format!("security-{}@example.invalid", Uuid::new_v4());
    enqueue_notification(&pool, &email).await;
    let port = std::env::var("AUTH_TEST_MAILPIT_SMTP_PORT")
        .unwrap()
        .parse()
        .unwrap();
    let sender = Arc::new(
        crate::providers::mailpit::MailpitEmailSender::new(
            "127.0.0.1".into(),
            port,
            "Sonimbus <noreply@example.invalid>".into(),
        )
        .unwrap(),
    );
    let service = SecurityNotificationService::new(
        SecurityNotificationsRepository { pool: pool.clone() },
        sender,
    );
    assert!(matches!(
        service.deliver_one().await.unwrap(),
        NotificationOutcome::Delivered
    ));
    let message = super::support::mailpit_message(&email).await;
    assert!(message["Text"].as_str().unwrap().contains("Google"));
    let html = message["HTML"].as_str().unwrap();
    assert!(!html.contains("href="));
    assert!(!html.contains("src="));
}
