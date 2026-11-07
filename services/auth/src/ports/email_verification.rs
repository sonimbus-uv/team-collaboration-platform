#[derive(Debug)]
pub enum EmailDeliveryError {
    Transport,
    Rejected(u16),
}

impl std::fmt::Display for EmailDeliveryError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => formatter.write_str("email provider transport failed"),
            Self::Rejected(status_code) => {
                write!(
                    formatter,
                    "email provider rejected request with HTTP {status_code}"
                )
            }
        }
    }
}

#[async_trait::async_trait]
pub trait VerificationEmailSender: Send + Sync {
    async fn send(
        &self,
        recipient_email: &str,
        code: &str,
        locale: &str,
    ) -> Result<(), EmailDeliveryError>;

    async fn send_recovery(
        &self,
        _recipient_email: &str,
        _code: &str,
        _locale: &str,
    ) -> Result<(), EmailDeliveryError> {
        Err(EmailDeliveryError::Transport)
    }
}

#[async_trait::async_trait]
pub trait RecipientBudget: Send + Sync {
    /// Returns true when allowed, false when exhausted, and an error when unavailable.
    async fn consume(&self, recipient_email: &str) -> Result<bool, ()>;
}

#[cfg(test)]
pub struct TestBudget;

#[cfg(test)]
#[async_trait::async_trait]
impl RecipientBudget for TestBudget {
    async fn consume(&self, _: &str) -> Result<bool, ()> {
        Ok(true)
    }
}
