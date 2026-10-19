use axum::http::Method;
use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthOperation {
    Register,
    Resend,
    Verify,
}

impl AuthOperation {
    pub fn from_request(method: &Method, path: &str) -> Option<Self> {
        match (method.as_str(), path) {
            ("POST", "/auth/register") => Some(Self::Register),
            ("POST", "/auth/resend-verification") => Some(Self::Resend),
            ("GET" | "POST", "/auth/verify-email") => Some(Self::Verify),
            _ => None,
        }
    }

    pub fn ip_key(self, ip: IpAddr) -> String {
        let operation = match self {
            Self::Register => "register",
            Self::Resend => "resend",
            Self::Verify => "verify",
        };

        format!("auth:rl:v1:{operation}:ip:{ip}")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct BucketPolicy {
    capacity: u32,
    refill_period_secs: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RateLimitDecision {
    Allowed,
    Limited { retry_after_secs: u64 },
}

impl BucketPolicy {
    pub fn new(capacity: u32, refill_period_secs: u32) -> anyhow::Result<Self> {
        anyhow::ensure!(
            (1..=10_000).contains(&capacity),
            "Bucket capacity must be between 1 and 10000"
        );

        anyhow::ensure!(
            (1..=86_400).contains(&refill_period_secs),
            "Bucket refill period must be between 1 and 86400 seconds"
        );

        Ok(Self {
            capacity,
            refill_period_secs,
        })
    }

    pub fn capacity(&self) -> u32 {
        self.capacity
    }

    pub fn refill_period_secs(&self) -> u32 {
        self.refill_period_secs
    }
}

#[cfg(test)]
mod tests {
    use super::{AuthOperation, BucketPolicy};
    use axum::http::Method;

    #[test]
    fn accepts_a_valid_policy() {
        let policy = BucketPolicy::new(6, 60).unwrap();

        assert_eq!(policy.capacity(), 6);
        assert_eq!(policy.refill_period_secs(), 60);
    }

    #[test]
    fn rejects_zero_capacity_or_refill_period() {
        assert!(BucketPolicy::new(0, 60).is_err());
        assert!(BucketPolicy::new(6, 0).is_err());
    }

    #[test]
    fn rejects_values_above_configuration_bounds() {
        assert!(BucketPolicy::new(10_001, 60).is_err());
        assert!(BucketPolicy::new(6, 86_401).is_err());
    }

        #[test]
    fn keys_separate_clients_and_operations() {
        let first_ip = "192.0.2.10".parse().unwrap();
        let second_ip = "192.0.2.11".parse().unwrap();

        assert_ne!(
            AuthOperation::Register.ip_key(first_ip),
            AuthOperation::Register.ip_key(second_ip)
        );

        assert_ne!(
            AuthOperation::Register.ip_key(first_ip),
            AuthOperation::Resend.ip_key(first_ip)
        );

        assert_ne!(
            AuthOperation::Resend.ip_key(first_ip),
            AuthOperation::Verify.ip_key(first_ip)
        );
    }

    #[test]
    fn get_and_post_verification_share_the_same_key() {
        let ip = "192.0.2.10".parse().unwrap();

        let get_operation =
            AuthOperation::from_request(&Method::GET, "/auth/verify-email")
                .unwrap();

        let post_operation =
            AuthOperation::from_request(&Method::POST, "/auth/verify-email")
                .unwrap();

        assert_eq!(get_operation.ip_key(ip), post_operation.ip_key(ip));
    }

    #[test]
    fn head_and_health_have_no_auth_operation() {
        assert_eq!(
            AuthOperation::from_request(&Method::HEAD, "/auth/verify-email"),
            None
        );

        for path in ["/health", "/health/db"] {
            assert_eq!(
                AuthOperation::from_request(&Method::GET, path),
                None
            );
        }
    }
}