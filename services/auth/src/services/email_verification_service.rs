use crate::{
    ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
    repositories::{
        accounts_repository::Account, one_time_tokens_repository::OneTimeTokensRepository,
    },
};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::sync::Arc;

pub fn new_token() -> (String, Vec<u8>) {
    let mut bytes = [0u8; 32];
    OsRng.fill_bytes(&mut bytes);
    let token: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
    let hash = Sha256::digest(token.as_bytes()).to_vec();
    (token, hash)
}

#[derive(Debug)]
pub enum VerificationError {
    InvalidToken,
    Database(sqlx::Error),
    Delivery(EmailDeliveryError),
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tokens_are_random_and_only_digest_is_persisted() {
        let (first, hash) = new_token();
        let (second, _) = new_token();
        assert_ne!(first, second);
        assert_eq!(first.len(), 64);
        assert_eq!(hash.len(), 32);
        assert_eq!(hash, Sha256::digest(first.as_bytes()).to_vec());
    }
}

#[derive(Clone)]
pub struct EmailVerificationService {
    repository: OneTimeTokensRepository,
    sender: Arc<dyn VerificationEmailSender>,
}

impl EmailVerificationService {
    pub fn new(
        repository: OneTimeTokensRepository,
        sender: Arc<dyn VerificationEmailSender>,
    ) -> Self {
        Self { repository, sender }
    }
    pub async fn send(&self, email: &str, token: &str) -> Result<(), EmailDeliveryError> {
        self.sender.send(email, token, "es-MX").await
    }
    pub async fn confirm(&self, token: &str) -> Result<Account, VerificationError> {
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(VerificationError::InvalidToken);
        }
        self.repository
            .confirm(&Sha256::digest(token.as_bytes()))
            .await
            .map_err(VerificationError::Database)?
            .ok_or(VerificationError::InvalidToken)
    }
    pub async fn resend(&self, email: &str) -> Result<(), VerificationError> {
        let (token, hash) = new_token();
        if self
            .repository
            .reissue(email, &hash)
            .await
            .map_err(VerificationError::Database)?
        {
            self.send(email, &token)
                .await
                .map_err(VerificationError::Delivery)?;
        }
        Ok(())
    }
}
