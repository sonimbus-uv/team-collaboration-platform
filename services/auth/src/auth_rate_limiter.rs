use std::{net::IpAddr, time::Duration};

use tokio::{sync::OnceCell, time::timeout};

use crate::{
    config::Config,
    rate_limit::{AuthOperation, BucketPolicy, RateLimitDecision},
    recipient_key::RecipientKey,
    redis_connection::{RedisConnection, RedisConnectionError},
};

pub struct AuthRateLimiter {
    #[cfg(test)]
    bypass_redis: bool,
    redis_url: String,
    connect_timeout: Duration,
    operation_timeout: Duration,
    connection: OnceCell<RedisConnection>,
    trusted_proxy_ips: Vec<IpAddr>,
    register_policy: BucketPolicy,
    resend_policy: BucketPolicy,
    verify_policy: BucketPolicy,
    recipient_policy: BucketPolicy,
    recipient_key: RecipientKey,
    login_policy: BucketPolicy,
    refresh_policy: BucketPolicy,
    logout_policy: BucketPolicy,
    login_account_policy: BucketPolicy,
}

impl AuthRateLimiter {
    pub fn from_config(config: &Config) -> anyhow::Result<Self> {
        redis::Client::open(config.redis_url.as_str())
            .map_err(|_| anyhow::anyhow!("Invalid Redis URL configuration"))?;

        let recipient_key = RecipientKey::new(config.recipient_hmac_secret.as_bytes())?;

        Ok(Self {
            #[cfg(test)]
            bypass_redis: false,
            redis_url: config.redis_url.clone(),
            connect_timeout: Duration::from_millis(u64::from(config.redis_connect_timeout_ms)),
            operation_timeout: Duration::from_millis(u64::from(config.redis_operation_timeout_ms)),
            connection: OnceCell::new(),
            trusted_proxy_ips: config.trusted_proxy_ips.clone(),
            register_policy: config.register_ip_policy,
            resend_policy: config.resend_ip_policy,
            verify_policy: config.verify_ip_policy,
            recipient_policy: config.recipient_policy,
            login_policy: config.login_ip_policy,
            refresh_policy: config.refresh_ip_policy,
            logout_policy: config.logout_ip_policy,
            login_account_policy: config.login_account_policy,
            recipient_key,
        })
    }

    #[cfg(test)]
    pub(crate) fn for_tests(redis_url: String) -> Self {
        let policy = BucketPolicy::new(10_000, 60).expect("The fixed test policy must be valid");

        Self {
            bypass_redis: false,
            redis_url,
            connect_timeout: Duration::from_millis(500),
            operation_timeout: Duration::from_millis(200),
            connection: OnceCell::new(),
            trusted_proxy_ips: Vec::new(),
            register_policy: policy,
            resend_policy: policy,
            verify_policy: policy,
            recipient_policy: policy,
            login_policy: policy,
            refresh_policy: policy,
            logout_policy: policy,
            login_account_policy: policy,
            recipient_key: RecipientKey::new(&[0x11; 32])
                .expect("The fixed test HMAC secret must be valid"),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_recipient_policy_for_tests(mut self, policy: BucketPolicy) -> Self {
        self.recipient_policy = policy;
        self
    }

    pub fn trusted_proxy_ips(&self) -> &[IpAddr] {
        &self.trusted_proxy_ips
    }

    #[cfg(test)]
    pub(crate) fn without_redis_for_tests(mut self) -> Self {
        self.bypass_redis = true;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_login_account_policy_for_tests(mut self, policy: BucketPolicy) -> Self {
        self.login_account_policy = policy;
        self
    }

    #[cfg(test)]
    pub(crate) fn with_ip_policy_for_tests(
        mut self,
        trusted_proxy_ips: Vec<IpAddr>,
        policy: BucketPolicy,
    ) -> Self {
        self.trusted_proxy_ips = trusted_proxy_ips;
        self.register_policy = policy;
        self.resend_policy = policy;
        self.verify_policy = policy;
        self.login_policy = policy;
        self.refresh_policy = policy;
        self.logout_policy = policy;
        self
    }

    pub async fn check_ip(
        &self,
        operation: AuthOperation,
        ip: IpAddr,
    ) -> Result<RateLimitDecision, RedisConnectionError> {
        let policy = self.policy_for(operation);
        #[cfg(test)]
        if self.bypass_redis {
            return Ok(RateLimitDecision::Allowed);
        }
        let key = operation.ip_key(ip);

        self.check_bucket(&key, policy).await
    }

    pub async fn check_recipient(
        &self,
        email: &str,
    ) -> Result<RateLimitDecision, RedisConnectionError> {
        let key = self.recipient_key.redis_key(email);

        self.check_bucket(&key, self.recipient_policy).await
    }

    pub async fn check_login_account(
        &self,
        email: &str,
    ) -> Result<RateLimitDecision, RedisConnectionError> {
        #[cfg(test)]
        if self.bypass_redis {
            return Ok(RateLimitDecision::Allowed);
        }
        self.check_bucket(
            &self.recipient_key.login_key(email),
            self.login_account_policy,
        )
        .await
    }

    async fn check_bucket(
        &self,
        key: &str,
        policy: BucketPolicy,
    ) -> Result<RateLimitDecision, RedisConnectionError> {
        let request_timeout = self.connect_timeout + self.operation_timeout;

        timeout(request_timeout, async {
            let connection = self
                .connection
                .get_or_try_init(|| async {
                    RedisConnection::connect(
                        &self.redis_url,
                        self.connect_timeout,
                        self.operation_timeout,
                    )
                    .await
                })
                .await?;

            connection.consume(key, policy).await
        })
        .await
        .map_err(|_| RedisConnectionError::OperationTimeout)?
    }

    pub async fn health_check(&self) -> Result<(), RedisConnectionError> {
        let request_timeout = self.connect_timeout + self.operation_timeout;
        timeout(request_timeout, async {
            let connection = self
                .connection
                .get_or_try_init(|| async {
                    RedisConnection::connect(
                        &self.redis_url,
                        self.connect_timeout,
                        self.operation_timeout,
                    )
                    .await
                })
                .await?;
            connection.ping().await
        })
        .await
        .map_err(|_| RedisConnectionError::OperationTimeout)?
    }

    fn policy_for(&self, operation: AuthOperation) -> BucketPolicy {
        match operation {
            AuthOperation::Login => self.login_policy,
            AuthOperation::Refresh => self.refresh_policy,
            AuthOperation::Logout => self.logout_policy,
            AuthOperation::Register => self.register_policy,
            AuthOperation::Resend => self.resend_policy,
            AuthOperation::Verify => self.verify_policy,
        }
    }
}

#[async_trait::async_trait]
impl crate::ports::email_verification::RecipientBudget for AuthRateLimiter {
    async fn consume(&self, email: &str) -> Result<bool, ()> {
        match self.check_recipient(email).await {
            Ok(RateLimitDecision::Allowed) => Ok(true),
            Ok(RateLimitDecision::Limited { .. }) => Ok(false),
            Err(error) => {
                tracing::warn!(
                    error_kind = ?error,
                    "Recipient budget check unavailable"
                );
                Err(())
            }
        }
    }
}
