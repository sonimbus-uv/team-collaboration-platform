use crate::email::verification_template::verification_email_content;
use crate::ports::email_verification::{EmailDeliveryError, VerificationEmailSender};
use lettre::{
    message::{Mailbox, MultiPart},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

pub struct MailpitEmailSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
    verification_url: reqwest::Url,
}

impl MailpitEmailSender {
    pub fn new(
        host: String,
        port: u16,
        from: String,
        verification_url: String,
    ) -> anyhow::Result<Self> {
        let verification_url = reqwest::Url::parse(&verification_url)?;
        anyhow::ensure!(
            verification_url.scheme() == "https"
                || (verification_url.scheme() == "http"
                    && matches!(verification_url.host_str(), Some("localhost" | "127.0.0.1"))),
            "EMAIL_VERIFICATION_URL must use HTTPS (HTTP allowed for localhost)"
        );
        Ok(Self {
            transport: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .port(port)
                .timeout(Some(std::time::Duration::from_secs(10)))
                .build(),
            from: from.parse()?,
            verification_url,
        })
    }
}

#[async_trait::async_trait]
impl VerificationEmailSender for MailpitEmailSender {
    async fn send(&self, email: &str, token: &str, locale: &str) -> Result<(), EmailDeliveryError> {
        let mut url = self.verification_url.clone();
        url.query_pairs_mut().append_pair("token", token);
        let content = verification_email_content(locale, url.as_str());
        let message = Message::builder()
            .from(self.from.clone())
            .to(email.parse().map_err(|_| EmailDeliveryError::Transport)?)
            .subject(content.subject)
            .multipart(MultiPart::alternative_plain_html(content.text, content.html))
            .map_err(|_| EmailDeliveryError::Transport)?;
        self.transport
            .send(message)
            .await
            .map_err(|_| EmailDeliveryError::Transport)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sender(url: &str, from: &str) -> anyhow::Result<MailpitEmailSender> {
        MailpitEmailSender::new("localhost".into(), 1025, from.into(), url.into())
    }

    #[tokio::test]
    async fn rejects_invalid_recipient_before_connecting() {
        let sender = sender("http://localhost:3001/auth/verify-email", "Team <team@example.com>")
            .unwrap();
        assert!(matches!(
            sender.send("invalid-address", "token", "es-MX").await,
            Err(EmailDeliveryError::Transport)
        ));
    }

    #[test]
    fn validates_sender_and_verification_url() {
        assert!(sender("http://example.com/verify", "team@example.com").is_err());
        assert!(sender("https://example.com/verify", "invalid-address").is_err());
        assert!(sender("https://example.com/verify", "Team <team@example.com>").is_ok());
    }
}
