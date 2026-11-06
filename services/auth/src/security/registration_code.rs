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

    fn mac(&self, registration_attempt_id: Uuid, code_version: i32, code: &str) -> Hmac<Sha256> {
        let mut mac = self.0.clone();

        mac.update(b"sonimbus:registration-code:v1\0");
        mac.update(registration_attempt_id.as_bytes());
        mac.update(&code_version.to_be_bytes());
        mac.update(code.as_bytes());

        mac
    }

    pub fn digest(&self, registration_attempt_id: Uuid, code_version: i32, code: &str) -> Vec<u8> {
        self.mac(registration_attempt_id, code_version, code)
            .finalize()
            .into_bytes()
            .to_vec()
    }

    pub fn verify(
        &self,
        registration_attempt_id: Uuid,
        code_version: i32,
        code: &str,
        expected_mac: &[u8],
    ) -> bool {
        // Always verify the MAC, including malformed code submissions.
        let code_format_valid = code.len() == 8 && code.bytes().all(|byte| byte.is_ascii_digit());

        self.mac(registration_attempt_id, code_version, code)
            .verify_slice(expected_mac)
            .is_ok()
            && code_format_valid
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_bound_to_attempt_version_purpose_and_secret() {
        let registration_key = RegistrationCodeKey::new(&[0x11; 32]).unwrap();
        let registration_attempt_id = Uuid::new_v4();
        let code = RegistrationCodeKey::generate();
        assert_eq!(code.len(), 8);

        let code_mac = registration_key.digest(registration_attempt_id, 0, &code);

        assert!(registration_key.verify(registration_attempt_id, 0, &code, &code_mac));
        assert!(!registration_key.verify(Uuid::new_v4(), 0, &code, &code_mac));
        assert!(!registration_key.verify(registration_attempt_id, 1, &code, &code_mac));
        assert!(!RegistrationCodeKey::new(&[0x22; 32]).unwrap().verify(
            registration_attempt_id,
            0,
            &code,
            &code_mac
        ));
        assert!(RegistrationCodeKey::new(&[0; 31]).is_err());
    }
    #[test]
    fn malformed_codes_are_rejected_even_with_a_matching_mac() {
        let key = RegistrationCodeKey::new(&[0x31; 32]).unwrap();
        let attempt_id = Uuid::new_v4();

        for code in [
            "",
            "1234567",
            "123456789",
            "1234abcd",
            " 1234567",
            "1234567 ",
            "1234\n678",
            "1234\t678",
            "1234\x00678",
            "\u{0661}234567",
        ] {
            let expected_mac = key.digest(attempt_id, 0, code);
            assert!(!key.verify(attempt_id, 0, code, &expected_mac));
        }
        let oversized_code = "1".repeat(4096);
        let expected_mac = key.digest(attempt_id, 0, &oversized_code);
        assert!(!key.verify(attempt_id, 0, &oversized_code, &expected_mac));
    }

    #[test]
    fn truncated_extended_and_tampered_macs_are_rejected() {
        let key = RegistrationCodeKey::new(&[0x31; 32]).unwrap();
        let attempt_id = Uuid::new_v4();

        let code = "01234567";
        let expected_mac = key.digest(attempt_id, 0, code);
        for length in [0, 1, 16, 31, 33, 64] {
            let mut malformed_mac = expected_mac.clone();
            malformed_mac.resize(length, 0);
            assert!(!key.verify(attempt_id, 0, code, &malformed_mac));
        }
        let mut tampered_mac = expected_mac.clone();
        tampered_mac[0] ^= 1;
        assert!(!key.verify(attempt_id, 0, code, &tampered_mac));
    }

    #[test]
    fn numeric_boundaries_and_leading_zeroes_are_accepted() {
        let key = RegistrationCodeKey::new(&[0x31; 32]).unwrap();
        let attempt_id = Uuid::new_v4();

        for code in ["00000000", "99999999", "01234567"] {
            let expected_mac = key.digest(attempt_id, 0, code);
            assert!(key.verify(attempt_id, 0, code, &expected_mac));
        }
    }
}
