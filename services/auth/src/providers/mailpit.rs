use crate::email::verification_template::verification_email_content;
use crate::ports::email_verification::{EmailDeliveryError, VerificationEmailSender};
use lettre::{
    message::{Mailbox, MultiPart},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

pub struct MailpitEmailSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: Mailbox,
}

impl MailpitEmailSender {
    pub fn new(host: String, port: u16, from: String) -> anyhow::Result<Self> {
        Ok(Self {
            transport: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .port(port)
                .timeout(Some(std::time::Duration::from_secs(10)))
                .build(),
            from: from.parse()?,
        })
    }
}

#[async_trait::async_trait]
impl VerificationEmailSender for MailpitEmailSender {
    async fn send(&self, email: &str, code: &str, locale: &str) -> Result<(), EmailDeliveryError> {
        let content = verification_email_content(locale, code);
        let message = Message::builder()
            .from(self.from.clone())
            .to(email.parse().map_err(|_| EmailDeliveryError::Transport)?)
            .subject(content.subject)
            .multipart(MultiPart::alternative_plain_html(
                content.text,
                content.html,
            ))
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

    fn sender(from: &str) -> anyhow::Result<MailpitEmailSender> {
        MailpitEmailSender::new("localhost".into(), 1025, from.into())
    }

    #[tokio::test]
    async fn rejects_invalid_recipient_before_connecting() {
        let sender = sender("Team <team@example.com>").unwrap();
        assert!(matches!(
            sender.send("invalid-address", "token", "es-MX").await,
            Err(EmailDeliveryError::Transport)
        ));
    }

    #[test]
    fn validates_sender() {
        assert!(sender("invalid-address").is_err());
        assert!(sender("Team <team@example.com>").is_ok());
    }
}
