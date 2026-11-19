use hmac::{Hmac, Mac};
use sha2::Sha256;
use uuid::Uuid;

#[derive(Clone)]
pub struct RecoveryCodeKey(Hmac<Sha256>);

impl RecoveryCodeKey {
    pub fn new(secret: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(
            secret.len() >= 32,
            "Recovery HMAC secret requires at least 32 bytes"
        );

        Ok(Self(
            Hmac::new_from_slice(secret).map_err(|_| anyhow::anyhow!("Invalid recovery key"))?,
        ))
    }

    fn mac(
        &self,
        recovery_attempt_id: Uuid,
        account_id: Uuid,
        credential_version: i64,
        code: &str,
    ) -> Hmac<Sha256> {
        let mut mac = self.0.clone();

        mac.update(b"sonimbus:password-reset:v1\0");
        mac.update(recovery_attempt_id.as_bytes());
        mac.update(account_id.as_bytes());
        mac.update(&credential_version.to_be_bytes());
        mac.update(code.as_bytes());

        mac
    }

    pub fn digest(
        &self,
        recovery_attempt_id: Uuid,
        account_id: Uuid,
        credential_version: i64,
        code: &str,
    ) -> Vec<u8> {
        self.mac(recovery_attempt_id, account_id, credential_version, code)
            .finalize()
            .into_bytes()
            .to_vec()
    }

    pub fn verify(
        &self,
        recovery_attempt_id: Uuid,
        account_id: Uuid,
        credential_version: i64,
        code: &str,
        expected_mac: &[u8],
    ) -> bool {
        let mac_matches = self
            .mac(recovery_attempt_id, account_id, credential_version, code)
            .verify_slice(expected_mac)
            .is_ok();

        mac_matches && code.len() == 8 && code.bytes().all(|byte| byte.is_ascii_digit())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn codes_are_bound_to_account_attempt_version_and_independent_key() {
        let recovery_key = RecoveryCodeKey::new(&[1; 32]).unwrap();
        let other_recovery_key = RecoveryCodeKey::new(&[2; 32]).unwrap();
        let recovery_attempt_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();
        let code_mac = recovery_key.digest(recovery_attempt_id, account_id, 10, "01234567");

        assert!(recovery_key.verify(recovery_attempt_id, account_id, 10, "01234567", &code_mac));
        assert!(!recovery_key.verify(
            recovery_attempt_id,
            Uuid::new_v4(),
            10,
            "01234567",
            &code_mac
        ));
        assert!(!recovery_key.verify(Uuid::new_v4(), account_id, 10, "01234567", &code_mac));
        assert!(!recovery_key.verify(recovery_attempt_id, account_id, 11, "01234567", &code_mac));
        assert!(!other_recovery_key.verify(
            recovery_attempt_id,
            account_id,
            10,
            "01234567",
            &code_mac
        ));
        assert!(RecoveryCodeKey::new(&[1; 31]).is_err());
    }
    #[test]
    fn malformed_codes_are_rejected_even_with_a_matching_mac() {
        let key = RecoveryCodeKey::new(&[0x31; 32]).unwrap();
        let attempt_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();

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
            let expected_mac = key.digest(attempt_id, account_id, 1, code);
            assert!(!key.verify(attempt_id, account_id, 1, code, &expected_mac));
        }
        let oversized_code = "1".repeat(4096);
        let expected_mac = key.digest(attempt_id, account_id, 1, &oversized_code);
        assert!(!key.verify(attempt_id, account_id, 1, &oversized_code, &expected_mac));
    }

    #[test]
    fn truncated_extended_and_tampered_macs_are_rejected() {
        let key = RecoveryCodeKey::new(&[0x31; 32]).unwrap();
        let attempt_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();

        let code = "01234567";
        let expected_mac = key.digest(attempt_id, account_id, 1, code);
        for length in [0, 1, 16, 31, 33, 64] {
            let mut malformed_mac = expected_mac.clone();
            malformed_mac.resize(length, 0);
            assert!(!key.verify(attempt_id, account_id, 1, code, &malformed_mac));
        }
        let mut tampered_mac = expected_mac.clone();
        tampered_mac[0] ^= 1;
        assert!(!key.verify(attempt_id, account_id, 1, code, &tampered_mac));
    }

    #[test]
    fn numeric_boundaries_and_leading_zeroes_are_accepted() {
        let key = RecoveryCodeKey::new(&[0x31; 32]).unwrap();
        let attempt_id = Uuid::new_v4();
        let account_id = Uuid::new_v4();

        for code in ["00000000", "99999999", "01234567"] {
            let expected_mac = key.digest(attempt_id, account_id, 1, code);
            assert!(key.verify(attempt_id, account_id, 1, code, &expected_mac));
        }
    }
}
