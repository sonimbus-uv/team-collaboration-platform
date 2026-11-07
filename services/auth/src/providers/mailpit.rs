use crate::{
    email::{
        recovery_template::recovery_email_content,
        verification_template::{verification_email_content, VerificationEmailContent},
    },
    ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
};
use lettre::{
    message::{Mailbox, MultiPart},
    AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};

pub struct MailpitEmailSender {
    smtp_transport: AsyncSmtpTransport<Tokio1Executor>,
    sender_address: Mailbox,
}

impl MailpitEmailSender {
    pub fn new(host: String, port: u16, sender_address: String) -> anyhow::Result<Self> {
        Ok(Self {
            smtp_transport: AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(host)
                .port(port)
                .timeout(Some(std::time::Duration::from_secs(10)))
                .build(),
            sender_address: sender_address.parse()?,
        })
    }

    async fn deliver(
        &self,
        recipient_email: &str,
        email_content: VerificationEmailContent,
    ) -> Result<(), EmailDeliveryError> {
        let smtp_message = Message::builder()
            .from(self.sender_address.clone())
            .to(recipient_email
                .parse()
                .map_err(|_| EmailDeliveryError::Transport)?)
            .subject(email_content.subject)
            .multipart(MultiPart::alternative_plain_html(
                email_content.text,
                email_content.html,
            ))
            .map_err(|_| EmailDeliveryError::Transport)?;

        self.smtp_transport
            .send(smtp_message)
            .await
            .map_err(|_| EmailDeliveryError::Transport)?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl VerificationEmailSender for MailpitEmailSender {
    async fn send(
        &self,
        recipient_email: &str,
        code: &str,
        locale: &str,
    ) -> Result<(), EmailDeliveryError> {
        self.deliver(recipient_email, verification_email_content(locale, code))
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

#[cfg(test)]
mod tests {
    use super::*;

    fn sender(sender_address: &str) -> anyhow::Result<MailpitEmailSender> {
        MailpitEmailSender::new("localhost".into(), 1025, sender_address.into())
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
