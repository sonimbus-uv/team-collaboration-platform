use axum::http::Method;
use std::net::IpAddr;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthOperation {
    ContactRead,
    ContactSend,
    ContactVerify,
    Register,
    Resend,
    Verify,
    Google,
    Reauthenticate,
    Login,
    Refresh,
    Logout,
    ChangePassword,
    ForgotPassword,
    ResetPassword,
}

impl AuthOperation {
    pub fn from_request(method: &Method, path: &str) -> Option<Self> {
        match (method.as_str(), path) {
            ("GET", "/api/v1/auth/contact-email") => Some(Self::ContactRead),
            (
                "POST",
                "/api/v1/auth/contact-email/verification-attempts"
                | "/api/v1/auth/contact-email/verification-resends",
            ) => Some(Self::ContactSend),
            ("POST", "/api/v1/auth/contact-email/verify") => Some(Self::ContactVerify),
            (
                "POST",
                "/api/v1/auth/google/attempts"
                | "/api/v1/auth/google/link-attempts"
                | "/api/v1/auth/google/exchange",
            )
            | (
                "GET",
                "/api/v1/auth/google/authorize"
                | "/api/v1/auth/google/callback"
                | "/api/v1/auth/connections",
            ) => Some(Self::Google),
            ("POST", "/api/v1/auth/reauthenticate") => Some(Self::Reauthenticate),
            ("POST", "/api/v1/auth/change-password") => Some(Self::ChangePassword),
            ("POST", "/api/v1/auth/forgot-password") => Some(Self::ForgotPassword),
            ("POST", "/api/v1/auth/reset-password") => Some(Self::ResetPassword),
            ("POST", "/api/v1/auth/login") => Some(Self::Login),
            ("POST", "/api/v1/auth/refresh") => Some(Self::Refresh),
            ("POST", "/api/v1/auth/logout") => Some(Self::Logout),
            ("POST", "/api/v1/auth/register") => Some(Self::Register),
            ("POST", "/api/v1/auth/resend-verification") => Some(Self::Resend),
            ("GET" | "POST", "/api/v1/auth/verify-email") => Some(Self::Verify),
            _ => None,
        }
    }

    pub fn ip_key(self, client_ip: IpAddr) -> String {
        let operation_name = match self {
            Self::ChangePassword => "change-password",
            Self::ForgotPassword => "forgot-password",
            Self::ResetPassword => "reset-password",
            Self::Google => "google",
            Self::Reauthenticate => "reauthenticate",
            Self::Login => "login",
            Self::Refresh => "refresh",
            Self::Logout => "logout",
            Self::ContactRead => "contact-read",
            Self::ContactSend => "contact-send",
            Self::ContactVerify => "contact-verify",
            Self::Register => "register",
            Self::Resend => "resend",
            Self::Verify => "verify",
        };

        format!("auth:rl:v1:{operation_name}:ip:{client_ip}")
    }

    pub fn is_session(self) -> bool {
        matches!(
            self,
            Self::ContactRead
                | Self::ContactSend
                | Self::ContactVerify
                | Self::Google
                | Self::Reauthenticate
                | Self::Login
                | Self::Refresh
                | Self::Logout
                | Self::ChangePassword
                | Self::ForgotPassword
                | Self::ResetPassword
        )
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

#[derive(Clone, Copy)]
pub enum ContactAccountOperation {
    Send,
    Verify,
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
        let client_ip = "192.0.2.10".parse().unwrap();

        let get_operation =
            AuthOperation::from_request(&Method::GET, "/api/v1/auth/verify-email").unwrap();

        let post_operation =
            AuthOperation::from_request(&Method::POST, "/api/v1/auth/verify-email").unwrap();

        assert_eq!(
            get_operation.ip_key(client_ip),
            post_operation.ip_key(client_ip)
        );
    }

    #[test]
    fn google_steps_share_a_budget_independent_of_password_login() {
        let ip = "192.0.2.10".parse().unwrap();
        for (method, route) in [
            (Method::POST, "attempts"),
            (Method::GET, "authorize"),
            (Method::GET, "callback"),
            (Method::POST, "exchange"),
        ] {
            let operation =
                AuthOperation::from_request(&method, &format!("/api/v1/auth/google/{route}"))
                    .unwrap();
            assert_eq!(operation, AuthOperation::Google);
            assert_eq!(operation.ip_key(ip), AuthOperation::Google.ip_key(ip));
            assert_ne!(operation.ip_key(ip), AuthOperation::Login.ip_key(ip));
        }
        assert!(
            AuthOperation::from_request(&Method::HEAD, "/api/v1/auth/google/callback").is_none()
        );
    }

    #[test]
    fn head_and_health_have_no_auth_operation() {
        assert_eq!(
            AuthOperation::from_request(&Method::HEAD, "/api/v1/auth/verify-email"),
            None
        );

        for path in ["/health", "/health/db"] {
            assert_eq!(AuthOperation::from_request(&Method::GET, path), None);
        }
    }
}
