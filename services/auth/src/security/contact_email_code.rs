use uuid::Uuid;

use super::verification_code::VerificationCodeKey;

pub struct ContactCodeContext<'a> {
    pub account_id: Uuid,
    pub email: &'a str,
    pub attempt_id: Uuid,
    pub contact_version: i64,
    pub code_version: i32,
}

#[derive(Clone)]
pub struct ContactEmailCodeKey(VerificationCodeKey);

impl ContactEmailCodeKey {
    pub fn new(secret: &[u8]) -> anyhow::Result<Self> {
        Ok(Self(VerificationCodeKey::new(secret)?))
    }

    fn encode_context(context: &ContactCodeContext<'_>) -> Vec<u8> {
        let mut encoded_context = Vec::with_capacity(100 + context.email.len());
        for part in [
            b"sonimbus:contact-email-verification:v1\0".as_slice(),
            context.account_id.as_bytes(),
            &(context.email.len() as u32).to_be_bytes(),
            context.email.as_bytes(),
            context.attempt_id.as_bytes(),
            &context.contact_version.to_be_bytes(),
            &context.code_version.to_be_bytes(),
        ] {
            encoded_context.extend_from_slice(part);
        }
        encoded_context
    }

    pub fn digest(&self, context: &ContactCodeContext<'_>, code: &str) -> Vec<u8> {
        self.0.digest(&[&Self::encode_context(context)], code)
    }

    pub fn verify(&self, context: &ContactCodeContext<'_>, code: &str, expected: &[u8]) -> bool {
        self.0
            .verify(&[&Self::encode_context(context)], code, expected)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contact_codes_are_bound_to_every_context_field_and_reject_malformed_input() {
        let key = ContactEmailCodeKey::new(&[0x51; 32]).unwrap();
        let mut context = ContactCodeContext {
            account_id: Uuid::new_v4(),
            email: "person@example.invalid",
            attempt_id: Uuid::new_v4(),
            contact_version: 1,
            code_version: 0,
        };
        let mac = key.digest(&context, "00123456");
        assert!(key.verify(&context, "00123456", &mac));
        context.contact_version += 1;
        assert!(!key.verify(&context, "00123456", &mac));
        context.contact_version -= 1;
        context.code_version += 1;
        assert!(!key.verify(&context, "00123456", &mac));
        context.code_version -= 1;
        context.email = "other@example.invalid";
        assert!(!key.verify(&context, "00123456", &mac));
        context.email = "person@example.invalid";
        let account = context.account_id;
        context.account_id = Uuid::new_v4();
        assert!(!key.verify(&context, "00123456", &mac));
        context.account_id = account;
        context.attempt_id = Uuid::new_v4();
        assert!(!key.verify(&context, "00123456", &mac));
        for code in [
            "",
            "1234567",
            "123456789",
            "abcdefgh",
            "1234\n678",
            "\u{0661}234567",
        ] {
            let malformed_mac = key.digest(&context, code);
            assert!(!key.verify(&context, code, &malformed_mac));
        }
        let registration =
            super::super::registration_code::RegistrationCodeKey::new(&[0x51; 32]).unwrap();
        assert!(!key.verify(
            &context,
            "00123456",
            &registration.digest(context.attempt_id, 0, "00123456")
        ));
    }
}
