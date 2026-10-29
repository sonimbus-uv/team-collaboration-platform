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
use tokio::{net::TcpListener, sync::Semaphore, task::JoinSet, time::timeout};
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
        // Separate pool makes SQL statement_timeout/read-only specific to session checks.
        // It uses the same authoritative DATABASE_URL and never runs migrations.
        let options: PgConnectOptions = database_url
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid internal database configuration"))?;
        let sql_ms = config.sql_timeout.as_millis();
        let pool = PgPoolOptions::new()
            .max_connections(config.concurrent_per_service.min(10))
            .acquire_timeout(config.sql_timeout)
            .after_connect(move |connection, _| {
                Box::pin(async move {
                    sqlx::query(&format!("SET statement_timeout={sql_ms}"))
                        .execute(&mut *connection)
                        .await?;
                    sqlx::query("SET default_transaction_read_only=on")
                        .execute(connection)
                        .await?;
                    Ok(())
                })
            })
            .connect_with(options.disable_statement_logging())
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

    pub async fn run(self) -> anyhow::Result<()> {
        let slots = Arc::new(Semaphore::new(self.api.config.local_limit));
        let mut connections = JoinSet::new();
        loop {
            tokio::select! {
                accepted = self.listener.accept() => {
                    let (stream, peer): (_, SocketAddr) = accepted.map_err(|_| anyhow::anyhow!("Internal listener failed"))?;
                    if !self.api.allows(peer.ip()) { continue; }
                    // Take the connection permit before TLS parsing. No unbounded waiter tasks.
                    let Ok(permit) = slots.clone().try_acquire_owned() else { continue; };
                    let (acceptor, api) = (self.acceptor.clone(), self.api.clone());
                    connections.spawn(async move {
                        let _permit = permit;
                        let Ok(Ok(tls)) = timeout(api.config.handshake_timeout, acceptor.accept(stream)).await else { return; };
                        let service = api.router().layer(Extension(ConnectInfo(peer)));
                        let mut http = http1::Builder::new();
                        http.timer(TokioTimer::new()).header_read_timeout(api.config.header_timeout)
                            .max_headers(32).max_buf_size(16_384).keep_alive(false);
                        // HTTP/1.1, one request per connection; also bound slow response writes.
                        let _ = timeout(api.config.header_timeout + api.config.request_timeout,
                            http.serve_connection(TokioIo::new(tls), TowerToHyperService::new(service))).await;
                    });
                },
                _ = connections.join_next(), if !connections.is_empty() => {},
            }
        }
    }
}

pub(super) fn load_tls(config: &InternalConfig) -> anyhow::Result<TlsAcceptor> {
    let certificate = read_file(&config.certificate_path, false)?;
    let key = read_file(&config.key_path, true)?;
    let certificates = CertificateDer::pem_slice_iter(&certificate)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| anyhow::anyhow!("Invalid internal TLS certificate"))?;
    ensure!(!certificates.is_empty(), "Missing internal TLS certificate");
    let mut keys = PrivateKeyDer::pem_slice_iter(&key);
    let key = keys
        .next()
        .transpose()
        .map_err(|_| anyhow::anyhow!("Invalid internal TLS private key"))?
        .ok_or_else(|| anyhow::anyhow!("Missing internal TLS private key"))?;
    ensure!(keys.next().is_none(), "Ambiguous internal TLS keys");
    let mut tls =
        ServerConfig::builder_with_provider(Arc::new(rustls::crypto::ring::default_provider()))
            .with_safe_default_protocol_versions()
            .map_err(|_| anyhow::anyhow!("Invalid internal TLS versions"))?
            .with_no_client_auth()
            .with_single_cert(certificates, key)
            .map_err(|_| {
                anyhow::anyhow!("Invalid or mismatched internal TLS key and certificate")
            })?;
    tls.alpn_protocols = vec![b"http/1.1".to_vec()];
    Ok(TlsAcceptor::from(Arc::new(tls)))
}
