use super::verification_code::VerificationCodeKey;
use uuid::Uuid;

#[derive(Clone)]
pub struct RegistrationCodeKey(VerificationCodeKey);

impl RegistrationCodeKey {
    pub fn new(secret: &[u8]) -> anyhow::Result<Self> {
        Ok(Self(VerificationCodeKey::new(secret)?))
    }
    pub fn generate() -> zeroize::Zeroizing<String> {
        VerificationCodeKey::generate()
    }
    pub fn digest(&self, registration_attempt_id: Uuid, code_version: i32, code: &str) -> Vec<u8> {
        self.0.digest(
            &[
                b"sonimbus:registration-code:v1\0",
                registration_attempt_id.as_bytes(),
                &code_version.to_be_bytes(),
            ],
            code,
        )
    }
    pub fn verify(
        &self,
        registration_attempt_id: Uuid,
        code_version: i32,
        code: &str,
        expected_mac: &[u8],
    ) -> bool {
        self.0.verify(
            &[
                b"sonimbus:registration-code:v1\0",
                registration_attempt_id.as_bytes(),
                &code_version.to_be_bytes(),
            ],
            code,
            expected_mac,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_registration_mac_format_remains_compatible() {
        let key = RegistrationCodeKey::new(&[0x11; 32]).unwrap();
        let id = Uuid::parse_str("12345678-1234-4234-8234-123456789abc").unwrap();
        let actual: String = key
            .digest(id, 2, "00123456")
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        assert_eq!(
            actual,
            "26c14257a826bf335e1375937a949259dd29aea8cf7bea899d801ebbd43b2e58"
        );
    }

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
