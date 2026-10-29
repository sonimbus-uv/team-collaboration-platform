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
        self.admit(
            format!("{}:ip:{{{ip}}}", self.prefix()),
            self.config.preauth_per_second,
            self.config.preauth_per_second,
            1,
            0,
            "",
        )
        .await
    }

    pub async fn reserve(
        self: &Arc<Self>,
        identity: Identity,
        pairs: usize,
    ) -> Result<Reservation, AdmissionError> {
        let mut reservation = Reservation {
            limits: self.clone(),
            identity,
            id: Uuid::new_v4().to_string(),
            released: false,
        };
        // The guard exists before awaiting Redis: cancellation/lost reply still triggers cleanup.
        let result = self
            .admit(
                self.service_key(identity),
                self.config.pair_capacity,
                self.config.pairs_per_second,
                pairs as u32,
                self.config.concurrent_per_service,
                &reservation.id,
            )
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

    async fn admit(
        &self,
        key: String,
        capacity: u32,
        rate: u32,
        cost: u32,
        concurrent: u32,
        id: &str,
    ) -> Result<(), AdmissionError> {
        let connection = self.connection().await?;
        let result = connection
            .script(
                &redis::Script::new(include_str!("admission.lua")),
                &[format!("{key}:quota"), format!("{key}:leases")],
                &[
                    capacity.to_string(),
                    rate.to_string(),
                    cost.to_string(),
                    concurrent.to_string(),
                    self.config.lease_ttl.as_millis().to_string(),
                    id.into(),
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

    async fn release(&self, identity: Identity, id: &str) -> Result<(), AdmissionError> {
        let removed = self
            .connection()
            .await?
            .script(
                &redis::Script::new("local score=redis.call('ZSCORE',KEYS[1],ARGV[1]); local t=redis.call('TIME'); local now=tonumber(t[1])*1000+math.floor(tonumber(t[2])/1000); redis.call('ZREM',KEYS[1],ARGV[1]); if score and tonumber(score)>now then return {1} else return {0} end"),
                &[format!("{}:leases", self.service_key(identity))],
                &[id.into()],
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
    id: String,
    released: bool,
}

impl Reservation {
    pub async fn release(mut self) -> Result<(), AdmissionError> {
        self.limits.release(self.identity, &self.id).await?;
        self.released = true;
        Ok(())
    }
}

impl Drop for Reservation {
    fn drop(&mut self) {
        if !self.released {
            let (limits, identity, id) = (self.limits.clone(), self.identity, self.id.clone());
            tokio::spawn(async move {
                let _ = limits.release(identity, &id).await;
            });
            // Expiring Redis leases bound capacity loss if release fails or the process dies.
        }
    }
}
