use crate::email::verification_template::verification_email_content;
use crate::ports::email_verification::{EmailDeliveryError, VerificationEmailSender};

pub struct ResendEmailSender {
    client: reqwest::Client,
    api_key: String,
    from: String,
    verification_url: reqwest::Url,
}

impl ResendEmailSender {
    pub fn new(api_key: String, from: String, verification_url: String) -> anyhow::Result<Self> {
        let verification_url = reqwest::Url::parse(&verification_url)?;
        anyhow::ensure!(
            verification_url.scheme() == "https"
                || (verification_url.scheme() == "http"
                    && matches!(verification_url.host_str(), Some("localhost" | "127.0.0.1"))),
            "EMAIL_VERIFICATION_URL must use HTTPS (HTTP allowed for localhost)"
        );
        anyhow::ensure!(
            !api_key.trim().is_empty() && !from.trim().is_empty(),
            "Resend credentials and sender must not be empty"
        );
        Ok(Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(10))
                .build()?,
            api_key,
            from,
            verification_url,
        })
    }
}

#[async_trait::async_trait]
impl VerificationEmailSender for ResendEmailSender {
    async fn send(&self, email: &str, token: &str, locale: &str) -> Result<(), EmailDeliveryError> {
        let mut url = self.verification_url.clone();
        url.query_pairs_mut().append_pair("token", token);
        let content = verification_email_content(locale, url.as_str());
        let response = self
            .client
            .post("https://api.resend.com/emails")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "from": self.from,
                "to": [email],
                "subject": content.subject,
                "text": content.text,
                "html": content.html
            }))
            .send()
            .await
            .map_err(|_| EmailDeliveryError::Transport)?;
        if !response.status().is_success() {
            return Err(EmailDeliveryError::Rejected(response.status().as_u16()));
        }
        Ok(())
    }
}
