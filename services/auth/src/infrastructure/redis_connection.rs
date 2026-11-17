use crate::protection::rate_limit::{BucketPolicy, RateLimitDecision};
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use std::{future::Future, time::Duration};
use tokio::time::timeout;

#[derive(Clone)]
pub struct RedisConnection {
    manager: ConnectionManager,
    operation_timeout: Duration,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RedisConnectionError {
    InvalidUrl,
    ConnectionTimeout,
    ConnectionFailed,
    OperationTimeout,
    OperationFailed,
    UnexpectedResponse,
}

impl RedisConnection {
    pub async fn script(
        &self,
        script: &redis::Script,
        keys: &[String],
        arguments: &[String],
    ) -> Result<Vec<i64>, RedisConnectionError> {
        let mut connection = self.manager.clone();
        let mut invocation = script.prepare_invoke();
        for key in keys {
            invocation.key(key);
        }
        for argument in arguments {
            invocation.arg(argument);
        }
        execute_operation(
            self.operation_timeout,
            invocation.invoke_async(&mut connection),
        )
        .await
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
        .map_err(|error| {
            if error.is_timeout() {
                RedisConnectionError::ConnectionTimeout
            } else {
                RedisConnectionError::ConnectionFailed
            }
        })?;

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

        let refill_period_ms = u64::from(policy.refill_period_secs()) * 1000;

        let mut invocation = script.prepare_invoke();
        invocation
            .key(key)
            .arg(policy.capacity())
            .arg(refill_period_ms);

        let response = execute_operation(
            self.operation_timeout,
            invocation.invoke_async(&mut connection),
        )
        .await?;

        decode_rate_limit_decision(response)
    }

    pub async fn ping(&self) -> Result<(), RedisConnectionError> {
        let mut connection = self.manager.clone();

        let response: String = execute_operation(
            self.operation_timeout,
            redis::cmd("PING").query_async(&mut connection),
        )
        .await?;

        if response != "PONG" {
            return Err(RedisConnectionError::UnexpectedResponse);
        }

        Ok(())
    }
}

async fn execute_operation<T>(
    operation_timeout: Duration,
    operation: impl Future<Output = redis::RedisResult<T>>,
) -> Result<T, RedisConnectionError> {
    timeout(operation_timeout, operation)
        .await
        .map_err(|_| RedisConnectionError::OperationTimeout)?
        .map_err(|error| {
            if error.is_timeout() {
                RedisConnectionError::OperationTimeout
            } else {
                tracing::warn!(error_kind = ?error.kind(), "Redis operation failed");
                RedisConnectionError::OperationFailed
            }
        })
}

fn decode_rate_limit_decision(
    (allowed, retry_after_ms): (i64, i64),
) -> Result<RateLimitDecision, RedisConnectionError> {
    match (allowed, retry_after_ms) {
        (1, 0) => Ok(RateLimitDecision::Allowed),
        (0, milliseconds) if milliseconds > 0 => Ok(RateLimitDecision::Limited {
            retry_after_secs: (milliseconds as u64).div_ceil(1_000),
        }),
        _ => Err(RedisConnectionError::UnexpectedResponse),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_connection() -> RedisConnection {
        let redis_url = std::env::var("REDIS_URL")
            .expect("REDIS_URL must point to a dedicated test Redis instance");
        RedisConnection::connect(&redis_url, Duration::from_secs(2), Duration::from_secs(2))
            .await
            .expect("Test Redis connection failed")
    }

    #[test]
    fn rate_limit_decisions_validate_status_and_round_retry_delays_up() {
        assert_eq!(
            decode_rate_limit_decision((1, 0)),
            Ok(RateLimitDecision::Allowed)
        );
        for (milliseconds, seconds) in [
            (1, 1),
            (999, 1),
            (1_000, 1),
            (1_001, 2),
            (i64::MAX, (i64::MAX as u64).div_ceil(1_000)),
        ] {
            assert_eq!(
                decode_rate_limit_decision((0, milliseconds)),
                Ok(RateLimitDecision::Limited {
                    retry_after_secs: seconds
                })
            );
        }
        for response in [(0, 0), (0, -1), (1, 1), (1, -1), (-1, 0), (2, 0)] {
            assert_eq!(
                decode_rate_limit_decision(response),
                Err(RedisConnectionError::UnexpectedResponse)
            );
        }
    }

    #[tokio::test]
    async fn operations_distinguish_deadlines_from_redis_failures() {
        let deadline = Duration::from_millis(10);
        assert_eq!(
            execute_operation::<()>(deadline, std::future::pending()).await,
            Err(RedisConnectionError::OperationTimeout)
        );
        let redis_timeout = redis::RedisError::from(std::io::Error::new(
            std::io::ErrorKind::TimedOut,
            "test timeout",
        ));
        assert_eq!(
            execute_operation::<()>(deadline, async { Err(redis_timeout) }).await,
            Err(RedisConnectionError::OperationTimeout)
        );
        let failure = redis::RedisError::from((redis::ErrorKind::ResponseError, "test failure"));
        assert_eq!(
            execute_operation::<()>(deadline, async { Err(failure) }).await,
            Err(RedisConnectionError::OperationFailed)
        );
        assert_eq!(execute_operation(deadline, async { Ok(42) }).await, Ok(42));
    }

    #[tokio::test]
    #[ignore = "requires REDIS_URL pointing to a dedicated test Redis instance"]
    async fn redis_responds_to_ping_and_enforces_bucket_capacity() {
        let connection = test_connection().await;

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
    #[ignore = "requires REDIS_URL pointing to a dedicated test Redis instance"]
    async fn redis_bucket_refills_and_expires() {
        let first = test_connection().await;
        let second = test_connection().await;
        let operation_timeout = first.operation_timeout;

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
    }

    #[tokio::test]
    #[ignore = "requires REDIS_URL pointing to a dedicated test Redis instance"]
    async fn redis_connections_share_concurrent_bucket_capacity() {
        let first = test_connection().await;
        let second = test_connection().await;
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
