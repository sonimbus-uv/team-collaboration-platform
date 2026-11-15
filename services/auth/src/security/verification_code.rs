use hmac::{Hmac, Mac};
use rand::{rngs::OsRng, Rng};
use sha2::Sha256;
use zeroize::Zeroizing;

const MIN_SECRET_BYTES: usize = 32;
const CODE_DIGITS: usize = 8;
const CODE_UPPER_BOUND: u32 = 100_000_000;

#[derive(Clone)]
pub struct VerificationCodeKey(Hmac<Sha256>);

impl VerificationCodeKey {
    pub fn new(secret: &[u8]) -> anyhow::Result<Self> {
        anyhow::ensure!(
            secret.len() >= MIN_SECRET_BYTES,
            "Verification HMAC secret requires at least 32 bytes"
        );
        Ok(Self(Hmac::new_from_slice(secret).map_err(|_| {
            anyhow::anyhow!("Invalid verification key")
        })?))
    }

    pub fn generate() -> Zeroizing<String> {
        Zeroizing::new(format!("{:08}", OsRng.gen_range(0..CODE_UPPER_BOUND)))
    }

    fn mac(&self, context: &[&[u8]], code: &str) -> Hmac<Sha256> {
        let mut mac = self.0.clone();
        for part in context {
            mac.update(part);
        }
        mac.update(code.as_bytes());
        mac
    }

    pub fn digest(&self, context: &[&[u8]], code: &str) -> Vec<u8> {
        self.mac(context, code).finalize().into_bytes().to_vec()
    }

    pub fn verify(&self, context: &[&[u8]], code: &str, expected: &[u8]) -> bool {
        let mac_matches = self.mac(context, code).verify_slice(expected).is_ok();
        mac_matches && code.len() == CODE_DIGITS && code.bytes().all(|byte| byte.is_ascii_digit())
    }
}
