use std::{net::IpAddr, sync::Arc};
use tokio::sync::OnceCell;
use uuid::Uuid;

use super::{config::InternalConfig, credentials::Identity};
use crate::infrastructure::redis_connection::RedisConnection;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionError {
    Unavailable,
    Limited(u64),
}

struct AdmissionRequest<'a> {
    bucket_key: String,
    capacity: u32,
    refill_per_second: u32,
    pair_cost: u32,
    concurrent_limit: u32,
    reservation_id: &'a str,
}

pub struct InternalLimits {
    config: InternalConfig,
    redis_url: String,
    connection: OnceCell<RedisConnection>,
    #[cfg(test)]
    pub(crate) test_namespace: String,
}

impl InternalLimits {
    pub fn new(config: InternalConfig, redis_url: String) -> anyhow::Result<Arc<Self>> {
        redis::Client::open(redis_url.as_str())
            .map_err(|_| anyhow::anyhow!("Invalid internal Redis configuration"))?;
        Ok(Arc::new(Self {
            config,
            redis_url,
            connection: OnceCell::new(),
            #[cfg(test)]
            test_namespace: format!("auth:internal:test:{}", Uuid::new_v4()),
        }))
    }

    async fn connection(&self) -> Result<&RedisConnection, AdmissionError> {
        self.connection
            .get_or_try_init(|| {
                RedisConnection::connect(
                    &self.redis_url,
                    self.config.redis_connect_timeout,
                    self.config.redis_operation_timeout,
                )
            })
            .await
            .map_err(|_| AdmissionError::Unavailable)
    }

    pub async fn preauthenticate(&self, ip: IpAddr) -> Result<(), AdmissionError> {
        // IP comes from the socket and passed the CIDR allow-list before reaching here.
        self.admit(AdmissionRequest {
            bucket_key: format!("{}:ip:{{{ip}}}", self.prefix()),
            capacity: self.config.preauth_per_second,
            refill_per_second: self.config.preauth_per_second,
            pair_cost: 1,
            concurrent_limit: 0,
            reservation_id: "",
        })
        .await
    }

    pub async fn reserve(
        self: &Arc<Self>,
        identity: Identity,
        pair_count: usize,
    ) -> Result<Reservation, AdmissionError> {
        let pair_cost = u32::try_from(pair_count).map_err(|_| AdmissionError::Unavailable)?;
        if pair_cost == 0 {
            return Err(AdmissionError::Unavailable);
        }

        let mut reservation = Reservation {
            limits: self.clone(),
            identity,
            reservation_id: Uuid::new_v4().to_string(),
            released: false,
        };
        // The guard exists before awaiting Redis: cancellation/lost reply still triggers cleanup.
        let result = self
            .admit(AdmissionRequest {
                bucket_key: self.service_key(identity),
                capacity: self.config.pair_capacity,
                refill_per_second: self.config.pairs_per_second,
                pair_cost,
                concurrent_limit: self.config.concurrent_per_service,
                reservation_id: &reservation.reservation_id,
            })
            .await;
        match result {
            Ok(()) => Ok(reservation),
            Err(error) => {
                // A definite rejection did not create a lease. An uncertain error uses Drop cleanup.
                if matches!(error, AdmissionError::Limited(_)) {
                    reservation.released = true;
                }
                Err(error)
            }
        }
    }

    async fn admit(&self, request: AdmissionRequest<'_>) -> Result<(), AdmissionError> {
        let connection = self.connection().await?;
        let result = connection
            .script(
                &redis::Script::new(include_str!("admission.lua")),
                &[
                    format!("{}:quota", request.bucket_key),
                    format!("{}:leases", request.bucket_key),
                ],
                &[
                    request.capacity.to_string(),
                    request.refill_per_second.to_string(),
                    request.pair_cost.to_string(),
                    request.concurrent_limit.to_string(),
                    self.config.lease_ttl.as_millis().to_string(),
                    request.reservation_id.into(),
                ],
            )
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        match result.as_slice() {
            [1, 0] => Ok(()),
            [0, retry] if *retry > 0 => Err(AdmissionError::Limited(*retry as u64)),
            _ => Err(AdmissionError::Unavailable),
        }
    }

    async fn release(
        &self,
        identity: Identity,
        reservation_id: &str,
    ) -> Result<(), AdmissionError> {
        let removed = self
            .connection()
            .await?
            .script(
                &redis::Script::new(include_str!("release.lua")),
                &[format!("{}:leases", self.service_key(identity))],
                &[reservation_id.into()],
            )
            .await
            .map_err(|_| AdmissionError::Unavailable)?;
        if removed.as_slice() == [1] {
            Ok(())
        } else {
            Err(AdmissionError::Unavailable)
        }
    }

    fn prefix(&self) -> &str {
        #[cfg(test)]
        {
            &self.test_namespace
        }
        #[cfg(not(test))]
        {
            "auth:internal:v1"
        }
    }

    fn service_key(&self, identity: Identity) -> String {
        format!("{}:service:{{{}}}", self.prefix(), identity.as_str())
    }
}

pub struct Reservation {
    limits: Arc<InternalLimits>,
    identity: Identity,
    reservation_id: String,
    released: bool,
}

impl Reservation {
    pub async fn release(mut self) -> Result<(), AdmissionError> {
        self.limits
            .release(self.identity, &self.reservation_id)
            .await?;
        self.released = true;
        Ok(())
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if !self.released {
            let (limits, identity, id) = (
                self.limits.clone(),
                self.identity,
                self.reservation_id.clone(),
            );
            if let Ok(runtime) = tokio::runtime::Handle::try_current() {
                runtime.spawn(async move {
                    let _ = limits.release(identity, &id).await;
                });
            }
            // The lease TTL bounds capacity loss if cleanup cannot run or Redis is unavailable.
        }
    }
}
