use crate::{
    email::{
        contact_template::google_link_notice, content::EmailContent,
        recovery_template::recovery_email_content,
        verification_template::verification_email_content,
    },
    ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
};

pub struct ResendEmailSender {
    http_client: reqwest::Client,
    api_key: String,
    sender_address: String,
}

impl ResendEmailSender {
    pub fn new(api_key: String, sender_address: String) -> anyhow::Result<Self> {
        anyhow::ensure!(
            !api_key.trim().is_empty() && !sender_address.trim().is_empty(),
            "Resend credentials and sender must not be empty"
        );

        Ok(Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?,
            api_key,
            sender_address,
        })
    }

    async fn deliver(
        &self,
        recipient_email: &str,
        email_content: EmailContent,
    ) -> Result<(), EmailDeliveryError> {
        let provider_response = self
            .http_client
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "from": self.sender_address,
                "to": [recipient_email],
                "subject": email_content.subject,
                "text": email_content.text,
                "html": email_content.html
            }))
            .send()
            .await
            .map_err(|_| EmailDeliveryError::Transport)?;

        if !provider_response.status().is_success() {
            return Err(EmailDeliveryError::Rejected(
                provider_response.status().as_u16(),
            ));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl VerificationEmailSender for ResendEmailSender {
    async fn send(
        &self,
        recipient_email: &str,
        code: &str,
        locale: &str,
    ) -> Result<(), EmailDeliveryError> {
        self.deliver(recipient_email, verification_email_content(locale, code))
            .await
    }

    async fn send_contact_verification(
        &self,
        recipient_email: &str,
        code: &str,
        locale: &str,
    ) -> Result<(), EmailDeliveryError> {
        self.deliver(recipient_email, verification_email_content(locale, code))
            .await
    }
    async fn send_security_notice(
        &self,
        recipient_email: &str,
        locale: &str,
    ) -> Result<(), EmailDeliveryError> {
        self.deliver(recipient_email, google_link_notice(locale))
            .await
    }

    async fn send_recovery(
        &self,
        recipient_email: &str,
        code: &str,
        locale: &str,
    ) -> Result<(), EmailDeliveryError> {
        self.deliver(recipient_email, recovery_email_content(locale, code))
            .await
    }
}
