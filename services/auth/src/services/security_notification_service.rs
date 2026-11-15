use std::{sync::Arc, time::Duration};

use crate::{
    ports::email_verification::VerificationEmailSender,
    repositories::security_notifications_repository::{
        SecurityNotification, SecurityNotificationsRepository,
    },
};

const DELIVERY_TIMEOUT: Duration = Duration::from_secs(12);
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const MAX_NOTIFICATIONS_PER_POLL: usize = 20;
const MAX_DELIVERY_ATTEMPTS: i32 = 10;

pub enum NotificationOutcome {
    Idle,
    Delivered,
    Retried,
    Exhausted,
    Superseded,
}

#[derive(Clone)]
pub struct SecurityNotificationService {
    repository: SecurityNotificationsRepository,
    sender: Arc<dyn VerificationEmailSender>,
}

impl SecurityNotificationService {
    pub fn new(
        repository: SecurityNotificationsRepository,
        sender: Arc<dyn VerificationEmailSender>,
    ) -> Self {
        Self { repository, sender }
    }

    pub async fn deliver_one(&self) -> Result<NotificationOutcome, sqlx::Error> {
        let Some(notification) = self.repository.claim().await? else {
            return Ok(NotificationOutcome::Idle);
        };

        let result = tokio::time::timeout(
            DELIVERY_TIMEOUT,
            self.sender
                .send_security_notice(&notification.recipient_email, "es-MX"),
        )
        .await;
        match result {
            Ok(Ok(())) => Ok(if self.repository.delivered(&notification).await? {
                NotificationOutcome::Delivered
            } else {
                NotificationOutcome::Superseded
            }),
            Ok(Err(_)) => {
                self.retry_notification(&notification, "provider_failed")
                    .await
            }
            Err(_) => self.retry_notification(&notification, "timeout").await,
        }
    }

    async fn retry_notification(
        &self,
        notification: &SecurityNotification,
        error_kind: &str,
    ) -> Result<NotificationOutcome, sqlx::Error> {
        if !self.repository.retry(notification, error_kind).await? {
            return Ok(NotificationOutcome::Superseded);
        }
        if notification.attempt_count >= MAX_DELIVERY_ATTEMPTS {
            Ok(NotificationOutcome::Exhausted)
        } else {
            Ok(NotificationOutcome::Retried)
        }
    }

    async fn process_batch(&self) {
        for _ in 0..MAX_NOTIFICATIONS_PER_POLL {
            match self.deliver_one().await {
                Ok(NotificationOutcome::Idle) => break,
                Ok(NotificationOutcome::Exhausted) => {
                    tracing::warn!("Security notification retry budget exhausted")
                }
                Ok(_) => {}
                Err(_) => {
                    tracing::warn!("Security notification processing unavailable");
                    break;
                }
            }
        }
    }

    pub fn spawn(self) -> tokio::task::JoinHandle<()> {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(POLL_INTERVAL);
            interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
            loop {
                interval.tick().await;
                self.process_batch().await;
            }
        })
    }
}
