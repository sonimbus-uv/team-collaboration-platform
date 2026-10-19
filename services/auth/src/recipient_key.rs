use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct RecipientKey {
    template: HmacSha256,
}

impl RecipientKey {
    pub fn new(secret: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(
            secret.len() >= 32,
            "Recipient HMAC secret must contain at least 32 bytes"
        );

        let template = HmacSha256::new_from_slice(secret)
            .map_err(|_| anyhow::anyhow!("Invalid recipient HMAC secret"))?;

        Ok(Self { template })
    }

    pub fn redis_key(&self, email: &str) -> String {
        let normalized_email = email.trim().to_lowercase();

        let mut mac = self.template.clone();
        mac.update(b"auth:email-verification:recipient:v1\0");
        mac.update(normalized_email.as_bytes());

        let digest = mac.finalize().into_bytes();

        let identifier: String = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();

        format!("auth:rl:v1:email-send:recipient:{identifier}")
    }
}

#[cfg(test)]
mod tests {
    use super::RecipientKey;

    #[test]
    fn normalization_produces_the_same_recipient_key() {
        let key = RecipientKey::new(&[0x11; 32]).unwrap();

        assert_eq!(
            key.redis_key(" PERSON@Example.COM "),
            key.redis_key("person@example.com")
        );
    }

    #[test]
    fn different_recipients_have_different_keys() {
        let key = RecipientKey::new(&[0x11; 32]).unwrap();

        assert_ne!(
            key.redis_key("first@example.com"),
            key.redis_key("second@example.com")
        );
    }

    #[test]
    fn different_secrets_produce_different_keys() {
        let first = RecipientKey::new(&[0x11; 32]).unwrap();
        let second = RecipientKey::new(&[0x22; 32]).unwrap();

        assert_ne!(
            first.redis_key("person@example.com"),
            second.redis_key("person@example.com")
        );
    }

    #[test]
    fn key_contains_a_digest_instead_of_the_email() {
        let generator = RecipientKey::new(&[0x11; 32]).unwrap();
        let key = generator.redis_key("person@example.com");

        let identifier = key
            .strip_prefix("auth:rl:v1:email-send:recipient:")
            .unwrap();

        assert_eq!(identifier.len(), 64);
        assert!(
            identifier
                .bytes()
                .all(|byte| byte.is_ascii_hexdigit())
        );
        assert!(!key.contains("person@example.com"));
        assert!(!key.contains('@'));
    }

    #[test]
    fn rejects_a_short_secret() {
        assert!(RecipientKey::new(&[0x11; 31]).is_err());
    }
}