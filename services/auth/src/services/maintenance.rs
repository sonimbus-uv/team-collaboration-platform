use std::time::Duration;

use super::{
    contact_email_service::ContactEmailService, google_auth_service::GoogleAuthService,
    password_service::PasswordService,
};

const CLEANUP_INTERVAL: Duration = Duration::from_secs(3600);

pub struct CleanupServices {
    pub passwords: Option<PasswordService>,
    pub google: Option<GoogleAuthService>,
    pub contact_email: Option<ContactEmailService>,
}

pub fn spawn_cleanup(services: CleanupServices) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(CLEANUP_INTERVAL);
        interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            interval.tick().await;
            services.cleanup_once().await;
        }
    })
}

impl CleanupServices {
    async fn cleanup_once(&self) {
        if let Some(service) = &self.passwords {
            if service.cleanup().await.is_err() {
                tracing::warn!("Password recovery cleanup unavailable");
            }
        }
        if let Some(service) = &self.google {
            if service.cleanup().await.is_err() {
                tracing::warn!("Google attempt cleanup unavailable");
            }
        }
        if let Some(service) = &self.contact_email {
            if service.cleanup().await.is_err() {
                tracing::warn!("Contact verification cleanup unavailable");
            }
        }
    }
}
