#[derive(Debug)]
pub enum EmailDeliveryError {
    Transport,
    Rejected(u16),
}

impl std::fmt::Display for EmailDeliveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Transport => f.write_str("email provider transport failed"),
            Self::Rejected(status) => {
                write!(f, "email provider rejected request with HTTP {status}")
            }
        }
    }
}

#[async_trait::async_trait]
pub trait VerificationEmailSender: Send + Sync {
    async fn send(&self, email: &str, code: &str, locale: &str) -> Result<(), EmailDeliveryError>;
}

#[async_trait::async_trait]
pub trait RecipientBudget: Send + Sync {
    // true: allowed;
    // false: budget exhausted.
    // Err: could not be checked.
    async fn consume(&self, email: &str) -> Result<bool, ()>;
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
