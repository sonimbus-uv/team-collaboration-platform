use std::{net::SocketAddr, sync::Arc};

use anyhow::ensure;
use axum::{extract::ConnectInfo, Extension};
use hyper::server::conn::http1;
use hyper_util::{
    rt::{TokioIo, TokioTimer},
    service::TowerToHyperService,
};
use rustls::pki_types::{pem::PemObject, CertificateDer, PrivateKeyDer};
use sqlx::{
    postgres::{PgConnectOptions, PgPoolOptions},
    ConnectOptions,
};
use tokio::{
    net::{TcpListener, TcpStream},
    sync::Semaphore,
    task::JoinSet,
    time::timeout,
};
use tokio_rustls::{
    rustls::{self, ServerConfig},
    TlsAcceptor,
};

use super::{
    config::{read_file, InternalConfig},
    credentials::ServiceCredentials,
    InternalApi,
};

pub struct InternalServer {
    listener: TcpListener,
    acceptor: TlsAcceptor,
    api: Arc<InternalApi>,
}

impl InternalServer {
    pub async fn prepare(
        config: InternalConfig,
        redis_url: String,
        database_url: &str,
    ) -> anyhow::Result<Self> {
        config.validate()?;
        let credentials = ServiceCredentials::load(&config.credentials_path)?;
        let acceptor = load_tls(&config)?;
        // Isolate read-only session checks and their SQL timeout from the public pool.
        let database_options: PgConnectOptions = database_url
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid internal database configuration"))?;
        let sql_timeout_ms = config.sql_timeout.as_millis();
        let pool = PgPoolOptions::new()
            .max_connections(config.concurrent_per_service.min(10))
            .acquire_timeout(config.sql_timeout)
            .after_connect(move |connection, _| {
                Box::pin(async move {
                    sqlx::query(&format!("SET statement_timeout={sql_timeout_ms}"))
                        .execute(&mut *connection)
                        .await?;
                    sqlx::query("SET default_transaction_read_only=on")
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(database_options.disable_statement_logging())
            .await
            .map_err(|_| anyhow::anyhow!("Cannot connect internal session-check database"))?;
        let api = InternalApi::new(config.clone(), credentials, redis_url, pool)?;
        let listener = TcpListener::bind(config.addr)
            .await
            .map_err(|_| anyhow::anyhow!("Cannot bind internal TLS listener"))?;
        Ok(Self {
            listener,
            acceptor,
            api,
        })
    }

    #[cfg(test)]
    pub fn for_tests(listener: TcpListener, acceptor: TlsAcceptor, api: Arc<InternalApi>) -> Self {
        Self {
            listener,
            acceptor,
            api,
        }
    }

    pub async fn run(self) -> anyhow::Result<()> {
        let connection_slots = Arc::new(Semaphore::new(self.api.config.local_limit));
        let mut connections = JoinSet::new();

        loop {
            tokio::select! {
                accepted = self.listener.accept() => {
                    let (stream, peer_address) = accepted
                        .map_err(|_| anyhow::anyhow!("Internal listener failed"))?;
                    if !self.api.allows(peer_address.ip()) {
                        continue;
                    }

                    // Acquire before spawning so TLS waiters cannot grow without a bound.
                    let Ok(permit) = connection_slots.clone().try_acquire_owned() else {
                        continue;
                    };
                    let acceptor = self.acceptor.clone();
                    let api = self.api.clone();
                    connections.spawn(async move {
                        let _permit = permit;
                        serve_connection(stream, peer_address, acceptor, api).await;
                    });
                }
                completed = connections.join_next(), if !connections.is_empty() => {
                    if let Some(Err(error)) = completed {
                        tracing::warn!(
                            cancelled = error.is_cancelled(),
                            panicked = error.is_panic(),
                            "Internal connection task terminated unexpectedly"
                        );
                    }
                }
            }
        }
    }
}

async fn serve_connection(
    stream: TcpStream,
    peer_address: SocketAddr,
    acceptor: TlsAcceptor,
    api: Arc<InternalApi>,
) {
    let Ok(Ok(tls_stream)) = timeout(api.config.handshake_timeout, acceptor.accept(stream)).await
    else {
        return;
    };
    let service = api.router().layer(Extension(ConnectInfo(peer_address)));
    let mut http = http1::Builder::new();
    http.timer(TokioTimer::new())
        .header_read_timeout(api.config.header_timeout)
        .max_headers(32)
        .max_buf_size(16_384)
        .keep_alive(false);

    // Bound response writes as well as TLS and request parsing; one request per connection.
    let _ = timeout(
        api.config.header_timeout + api.config.request_timeout,
        http.serve_connection(TokioIo::new(tls_stream), TowerToHyperService::new(service)),
    )
    .await;
}

pub(super) fn load_tls(config: &InternalConfig) -> anyhow::Result<TlsAcceptor> {
    let certificate_pem = read_file(&config.certificate_path, false)?;
    let private_key_pem = read_file(&config.key_path, true)?;
    let certificates = CertificateDer::pem_slice_iter(&certificate_pem)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| anyhow::anyhow!("Invalid internal TLS certificate"))?;
    ensure!(!certificates.is_empty(), "Missing internal TLS certificate");
    let mut private_keys = PrivateKeyDer::pem_slice_iter(&private_key_pem);
    let private_key = private_keys
        .next()
        .transpose()
        .map_err(|_| anyhow::anyhow!("Invalid internal TLS private key"))?
        .ok_or_else(|| anyhow::anyhow!("Missing internal TLS private key"))?;
    ensure!(private_keys.next().is_none(), "Ambiguous internal TLS keys");
    let mut tls_config =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|_| anyhow::anyhow!("Invalid internal TLS versions"))?
            .with_no_client_auth()
            .with_single_cert(certificates, private_key)
            .map_err(|_| {
                anyhow::anyhow!("Invalid or mismatched internal TLS key and certificate")
            })?;
    tls_config.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(TlsAcceptor::from(Arc::new(tls_config)))
}
