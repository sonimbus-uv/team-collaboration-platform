use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

pub struct RecipientKey {
    mac_template: HmacSha256,
}

impl RecipientKey {
    pub fn login_key(&self, email: &str) -> String {
        let mut recipient_mac = self.mac_template.clone();

        recipient_mac.update(b"auth:login-account:v1\0");
        recipient_mac.update(email.trim().to_lowercase().as_bytes());

        let mac_digest = recipient_mac.finalize().into_bytes();
        let digest_identifier: String = mac_digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();

        format!("auth:rl:v1:login:account:{digest_identifier}")
    }

    pub fn new(secret: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(
            secret.len() >= 32,
            "Recipient HMAC secret must contain at least 32 bytes"
        );

        let mac_template = HmacSha256::new_from_slice(secret)
            .map_err(|_| anyhow::anyhow!("Invalid recipient HMAC secret"))?;

        Ok(Self { mac_template })
    }

    pub fn redis_key(&self, email: &str) -> String {
        let normalized_email = email.trim().to_lowercase();
        let mut recipient_mac = self.mac_template.clone();

        recipient_mac.update(b"auth:email-verification:recipient:v1\0");
        recipient_mac.update(normalized_email.as_bytes());

        let mac_digest = recipient_mac.finalize().into_bytes();
        let digest_identifier: String = mac_digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();

        format!("auth:rl:v1:email-send:recipient:{digest_identifier}")
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
        assert_eq!(
            key.login_key(" PERSON@Example.COM "),
            key.login_key("person@example.com")
        );
        assert_ne!(
            key.login_key("person@example.com"),
            key.redis_key("person@example.com")
        );
        assert!(!key
            .login_key("person@example.com")
            .contains("person@example.com"));
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
        let first_secret_key = RecipientKey::new(&[0x11; 32]).unwrap();
        let second_secret_key = RecipientKey::new(&[0x22; 32]).unwrap();

        assert_ne!(
            first_secret_key.redis_key("person@example.com"),
            second_secret_key.redis_key("person@example.com")
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
        assert!(identifier.bytes().all(|byte| byte.is_ascii_hexdigit()));
        assert!(!key.contains("person@example.com"));
        assert!(!key.contains('@'));
    }

    #[test]
    fn rejects_a_short_secret() {
        assert!(RecipientKey::new(&[0x11; 31]).is_err());
    }
}
