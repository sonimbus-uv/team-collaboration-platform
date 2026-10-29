use crate::protection::rate_limit::{BucketPolicy, RateLimitDecision};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use std::time::Duration;
use tokio::time::timeout;

#[derive(Clone)]
pub struct RedisConnection {
    manager: ConnectionManager,
    operation_timeout: Duration,
}

#[derive(Debug)]
pub enum RedisConnectionError {
    InvalidUrl,
    ConnectionTimeout,
    ConnectionFailed,
    OperationTimeout,
    OperationFailed,
    UnexpectedResponse,
}

impl RedisConnection {
    /// Execute a bounded internal-protection script using the existing connection manager.
    /// Errors remain finite categories; never log keys, arguments or Redis diagnostics.
    pub async fn script(
        &self,
        script: &redis::Script,
        keys: &[String],
        args: &[String],
    ) -> Result<Vec<i64>, RedisConnectionError> {
        let mut connection = self.manager.clone();
        let mut invocation = script.prepare_invoke();
        for key in keys {
            invocation.key(key);
        }
        for arg in args {
            invocation.arg(arg);
        }
        timeout(
            self.operation_timeout,
            invocation.invoke_async(&mut connection),
        )
        .await
        .map_err(|_| RedisConnectionError::OperationTimeout)?
        .map_err(|_| RedisConnectionError::OperationFailed)
    }

    pub async fn connect(
        url: &str,
        connect_timeout: Duration,
        operation_timeout: Duration,
    ) -> Result<Self, RedisConnectionError> {
        let client = redis::Client::open(url).map_err(|_| RedisConnectionError::InvalidUrl)?;

        let manager_config = ConnectionManagerConfig::new()
            .set_connection_timeout(connect_timeout)
            .set_response_timeout(operation_timeout)
            .set_number_of_retries(0);

        let manager = timeout(
            connect_timeout,
            ConnectionManager::new_with_config(client, manager_config),
        )
        .await
        .map_err(|_| RedisConnectionError::ConnectionTimeout)?
        .map_err(|_| RedisConnectionError::ConnectionFailed)?;

        Ok(Self {
            manager,
            operation_timeout,
        })
    }

    pub async fn consume(
        &self,
        key: &str,
        policy: BucketPolicy,
    ) -> Result<RateLimitDecision, RedisConnectionError> {
        let mut connection = self.manager.clone();
        let script = redis::Script::new(include_str!("../protection/rate_limit.lua"));

        let refill_ms = u64::from(policy.refill_period_secs()) * 1000;

        let mut invocation = script.prepare_invoke();
        invocation.key(key).arg(policy.capacity()).arg(refill_ms);

        let (allowed, retry_ms): (i64, i64) = timeout(
            self.operation_timeout,
            invocation.invoke_async(&mut connection),
        )
        .await
        .map_err(|_| RedisConnectionError::OperationTimeout)?
        .map_err(|error| {
            tracing::warn!(
                error_kind = ?error.kind(),
                "Redis rate limit operation failed"
            );

            RedisConnectionError::OperationFailed
        })?;

        match (allowed, retry_ms) {
            (1, 0) => Ok(RateLimitDecision::Allowed),
            (0, wait) if wait > 0 => {
                let retry_after_secs = (wait as u64).div_ceil(1000);

                Ok(RateLimitDecision::Limited { retry_after_secs })
            }
            _ => Err(RedisConnectionError::UnexpectedResponse),
        }
    }

    pub async fn ping(&self) -> Result<(), RedisConnectionError> {
        let mut connection = self.manager.clone();

        let response: String = timeout(
            self.operation_timeout,
            redis::cmd("PING").query_async(&mut connection),
        )
        .await
        .map_err(|_| RedisConnectionError::OperationTimeout)?
        .map_err(|error| {
            tracing::warn!(
                error_kind = ?error.kind(),
                is_timeout = error.is_timeout(),
                is_io_error = error.is_io_error(),
                "Redis operation failed"
            );

            RedisConnectionError::OperationFailed
        })?;

        if response != "PONG" {
            return Err(RedisConnectionError::UnexpectedResponse);
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "requires the local .env and a running Redis instance"]
    async fn configured_redis_responds_to_ping() {
        dotenvy::dotenv().expect("This manual test requires a valid local .env");

        let config =
            crate::config::Config::from_env().expect("The local Auth configuration must be valid");

        let connection = RedisConnection::connect(
            &config.redis_url,
            Duration::from_millis(u64::from(config.redis_connect_timeout_ms)),
            Duration::from_millis(u64::from(config.redis_operation_timeout_ms)),
        )
        .await
        .expect("Redis connection failed; check address, password and availability");

        connection
            .ping()
            .await
            .expect("Redis did not respond successfully within the configured timeout");

        let policy = BucketPolicy::new(2, 60).unwrap();

        let key = format!("auth:rl:test:{}", rand::random::<u128>());

        assert_eq!(
            connection.consume(&key, policy).await.unwrap(),
            RateLimitDecision::Allowed
        );

        assert_eq!(
            connection.consume(&key, policy).await.unwrap(),
            RateLimitDecision::Allowed
        );

        let decision = connection.consume(&key, policy).await.unwrap();

        match decision {
            RateLimitDecision::Limited { retry_after_secs } => {
                assert!(
                    (1..=30).contains(&retry_after_secs),
                    "unexpected retry delay: {retry_after_secs}"
                );
            }
            RateLimitDecision::Allowed => {
                panic!("The third immediate request must exceed the bucket capacity");
            }
        }
    }

    #[tokio::test]
    #[ignore = "requires the local .env and a running development Redis instance"]
    async fn redis_bucket_refills_expires_and_shares_concurrent_budget() {
        dotenvy::dotenv().expect("This manual test requires a valid local .env");

        let config =
            crate::config::Config::from_env().expect("The local Auth configuration must be valid");

        let connect_timeout = Duration::from_millis(u64::from(config.redis_connect_timeout_ms));
        let operation_timeout = Duration::from_millis(u64::from(config.redis_operation_timeout_ms));

        let first = RedisConnection::connect(&config.redis_url, connect_timeout, operation_timeout)
            .await
            .expect("First Redis connection failed");

        let second =
            RedisConnection::connect(&config.redis_url, connect_timeout, operation_timeout)
                .await
                .expect("Second Redis connection failed");

        let refill_key = format!("auth:rl:test:refill:{}", rand::random::<u128>());
        let refill_policy = BucketPolicy::new(2, 4).unwrap();

        assert_eq!(
            first.consume(&refill_key, refill_policy).await.unwrap(),
            RateLimitDecision::Allowed
        );
        assert_eq!(
            first.consume(&refill_key, refill_policy).await.unwrap(),
            RateLimitDecision::Allowed
        );
        assert!(matches!(
            first.consume(&refill_key, refill_policy).await.unwrap(),
            RateLimitDecision::Limited {
                retry_after_secs: 1..=2
            }
        ));

        tokio::time::sleep(Duration::from_millis(2_200)).await;

        let mut observer = second.manager.clone();
        let remaining_ttl: i64 = timeout(
            operation_timeout,
            redis::cmd("PTTL")
                .arg(&refill_key)
                .query_async(&mut observer),
        )
        .await
        .expect("PTTL timed out")
        .expect("PTTL failed");

        assert!(
            remaining_ttl > 0,
            "The bucket should still exist before its full refill"
        );

        assert_eq!(
            second.consume(&refill_key, refill_policy).await.unwrap(),
            RateLimitDecision::Allowed
        );

        tokio::time::sleep(Duration::from_millis(4_300)).await;

        let expired_ttl: i64 = timeout(
            operation_timeout,
            redis::cmd("PTTL")
                .arg(&refill_key)
                .query_async(&mut observer),
        )
        .await
        .expect("PTTL timed out")
        .expect("PTTL failed");

        assert_eq!(expired_ttl, -2, "The inactive bucket must expire");

        assert_eq!(
            first.consume(&refill_key, refill_policy).await.unwrap(),
            RateLimitDecision::Allowed
        );

        let concurrent_key = format!("auth:rl:test:concurrent:{}", rand::random::<u128>());
        let concurrent_policy = BucketPolicy::new(5, 60).unwrap();

        let barrier = std::sync::Arc::new(tokio::sync::Barrier::new(33));
        let mut tasks = tokio::task::JoinSet::new();

        for index in 0..32 {
            let connection = if index % 2 == 0 {
                first.clone()
            } else {
                second.clone()
            };

            let key = concurrent_key.clone();
            let barrier = barrier.clone();

            tasks.spawn(async move {
                barrier.wait().await;
                connection.consume(&key, concurrent_policy).await
            });
        }

        let admitted = timeout(Duration::from_secs(5), async {
            barrier.wait().await;

            let mut admitted = 0;

            while let Some(result) = tasks.join_next().await {
                let decision = result
                    .expect("Concurrent task failed")
                    .expect("Concurrent Redis operation failed");

                if decision == RateLimitDecision::Allowed {
                    admitted += 1;
                }
            }

            admitted
        })
        .await
        .expect("Concurrent test exceeded five seconds");

        assert_eq!(
            admitted, 5,
            "Both Redis connections must share the same five-token capacity"
        );
    }
}
