use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, Rng};
use sha2::Sha256;
use uuid::Uuid;

#[derive(Clone)]
pub struct RegistrationCodeKey(Hmac<Sha256>);

impl RegistrationCodeKey {
    pub fn new(secret: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(
            secret.len() >= 32,
            "Registration code HMAC secret requires at least 32 bytes"
        );
        Ok(Self(Hmac::new_from_slice(secret).map_err(|_| {
            anyhow::anyhow!("Invalid registration code key")
        })?))
    }

    pub fn generate() -> zeroize::Zeroizing<String> {
        zeroize::Zeroizing::new(format!("{:08}", OsRng.gen_range(0..100_000_000u32)))
    }

    fn mac(&self, id: Uuid, version: i32, code: &str) -> Hmac<Sha256> {
        let mut mac = self.0.clone();
        mac.update(b"sonimbus:registration-code:v1\0");
        mac.update(id.as_bytes());
        mac.update(&version.to_be_bytes());
        mac.update(code.as_bytes());
        mac
    }

    pub fn digest(&self, id: Uuid, version: i32, code: &str) -> Vec<u8> {
        self.mac(id, version, code).finalize().into_bytes().to_vec()
    }

    pub fn verify(&self, id: Uuid, version: i32, code: &str, expected: &[u8]) -> bool {
        // Invalid-format submissions to a real attempt count towards its persistent budget.
        let valid = code.len() == 8 && code.bytes().all(|b| b.is_ascii_digit());
        self.mac(id, version, code).verify_slice(expected).is_ok() && valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn codes_are_bound_to_attempt_version_purpose_and_secret() {
        let key = RegistrationCodeKey::new(&[0x11; 32]).unwrap();
        let id = Uuid::new_v4();
        let code = RegistrationCodeKey::generate();
        assert_eq!(code.len(), 8);
        let mac = key.digest(id, 0, &code);
        assert!(key.verify(id, 0, &code, &mac));
        assert!(!key.verify(Uuid::new_v4(), 0, &code, &mac));
        assert!(!key.verify(id, 1, &code, &mac));
        assert!(!RegistrationCodeKey::new(&[0x22; 32])
            .unwrap()
            .verify(id, 0, &code, &mac));
        assert!(RegistrationCodeKey::new(&[0; 31]).is_err());
    }
}
