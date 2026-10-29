//! Uses only sqlx-created databases, disposable credentials/certificates and unique Redis keys.
use super::*;
use axum::{
    body::{to_bytes, Body, Bytes},
    http::Request,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use sha2::{Digest, Sha256};
use sqlx::types::chrono::Utc;
use std::{
    convert::Infallible,
    path::PathBuf,
    pin::Pin,
    task::{Context, Poll},
    time::Duration,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tower::ServiceExt;
use uuid::Uuid;

fn settings() -> InternalConfig {
    InternalConfig {
        addr: "127.0.0.1:3443".parse().unwrap(),
        allowed_peers: vec!["127.0.0.1/32".parse().unwrap()],
        credentials_path: "unused.json".into(),
        certificate_path: "unused.pem".into(),
        key_path: "unused.key".into(),
        body_limit: 65_536,
        pair_capacity: 100,
        pairs_per_second: 100,
        concurrent_per_service: 20,
        preauth_per_second: 10_000,
        local_limit: 50,
        request_timeout: Duration::from_secs(1),
        sql_timeout: Duration::from_millis(500),
        redis_connect_timeout: Duration::from_millis(200),
        redis_operation_timeout: Duration::from_millis(100),
        lease_ttl: Duration::from_secs(2),
        handshake_timeout: Duration::from_millis(300),
        header_timeout: Duration::from_secs(1),
    }
}
fn token(seed: u8) -> String {
    format!("svc_{}", URL_SAFE_NO_PAD.encode([seed; 32]))
}
fn digest(seed: u8) -> String {
    Sha256::digest(token(seed).as_bytes())
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect()
}
fn document() -> serde_json::Value {
    serde_json::json!({
        "core":{"enabled":true,"permissions":["sessions:check"],"active_sha256":digest(1)},
        "files":{"enabled":true,"permissions":["sessions:check"],"active_sha256":digest(2)}
    })
}
fn redis_url() -> String {
    std::env::var("REDIS_URL").expect("Use an isolated REDIS_URL")
}
fn api(pool: PgPool, config: InternalConfig, document: serde_json::Value) -> Arc<InternalApi> {
    InternalApi::new(
        config,
        ServiceCredentials::parse(&serde_json::to_vec(&document).unwrap(), Utc::now()).unwrap(),
        redis_url(),
        pool,
    )
    .unwrap()
}
fn request(path: &str, credential: Option<&str>, body: Body) -> Request<Body> {
    let mut builder = Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .extension(ConnectInfo(
            "127.0.0.1:12345".parse::<SocketAddr>().unwrap(),
        ));
    if let Some(credential) = credential {
        builder = builder.header(header::AUTHORIZATION, format!("Bearer {credential}"));
    }
    builder.body(body).unwrap()
}
fn pair(sid: Uuid, account: Uuid) -> serde_json::Value {
    serde_json::json!({"sid":sid,"account_id":account})
}
async fn fixture(pool: &PgPool) -> serde_json::Value {
    let account: Uuid = sqlx::query_scalar("INSERT INTO auth.accounts(email,email_verified_at) VALUES($1,statement_timestamp()) RETURNING id")
        .bind(format!("{}@example.invalid",Uuid::new_v4())).fetch_one(pool).await.unwrap();
    let sid: Uuid = sqlx::query_scalar("INSERT INTO auth.sessions(account_id,client_type,created_at,expires_at) VALUES($1,'desktop',statement_timestamp(),statement_timestamp()+interval '1 day') RETURNING id")
        .bind(account).fetch_one(pool).await.unwrap();
    pair(sid, account)
}
async fn call(
    api: &Arc<InternalApi>,
    path: &str,
    credential: Option<&str>,
    body: serde_json::Value,
) -> Response {
    api.router()
        .oneshot(request(path, credential, Body::from(body.to_string())))
        .await
        .unwrap()
}
async fn assert_problem(response: Response, status: StatusCode, kind: &str) {
    assert_eq!(response.status(), status);
    assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
    assert_eq!(
        response.headers()[header::CONTENT_TYPE],
        "application/problem+json"
    );
    if status == StatusCode::UNAUTHORIZED {
        assert_eq!(response.headers()[header::WWW_AUTHENTICATE], "Bearer");
    }
    if status == StatusCode::TOO_MANY_REQUESTS {
        assert!(
            response.headers()[header::RETRY_AFTER]
                .to_str()
                .unwrap()
                .parse::<u64>()
                .unwrap()
                >= 1
        );
    }
    let bytes = to_bytes(response.into_body(), 4096).await.unwrap();
    let body: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(body["type"], format!("/problems/{kind}"));
    assert_eq!(body["status"], status.as_u16());
    for secret in [
        token(1),
        token(2),
        digest(1),
        "password".into(),
        "SELECT".into(),
        "postgres://".into(),
        "redis://".into(),
    ] {
        assert!(!String::from_utf8_lossy(&bytes).contains(&secret));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn http_checks_reuse_4a_and_both_service_identities(pool: PgPool) {
    let active = fixture(&pool).await;
    let missing = pair(Uuid::new_v4(), Uuid::new_v4());
    let api = api(pool, settings(), document());
    for seed in [1, 2] {
        let response = call(
            &api,
            "/internal/v1/sessions/check",
            Some(&token(seed)),
            active.clone(),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");
        let result: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(result["active"], true);
        assert_eq!(result["sid"], active["sid"]);
        assert_eq!(result.as_object().unwrap().len(), 4);
        let response = call(
            &api,
            "/internal/v1/sessions/check-batch",
            Some(&token(seed)),
            serde_json::json!({"sessions":[active.clone(),missing.clone()]}),
        )
        .await;
        assert_eq!(response.status(), StatusCode::OK);
        let batch: serde_json::Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 4096).await.unwrap()).unwrap();
        assert_eq!(batch["results"].as_array().unwrap().len(), 2);
        assert_eq!(batch["results"][0], result);
        assert_eq!(batch["results"][1]["active"], false);
        assert!(batch["results"][1]["session_expires_at"].is_null());
    }
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn http_rejects_missing_bad_jwt_retired_and_unpermitted_credentials(pool: PgPool) {
    let mut doc = document();
    doc["files"]["permissions"] = serde_json::json!([]);
    let api = api(pool, settings(), doc);
    let body = pair(Uuid::new_v4(), Uuid::new_v4());
    let now = Utc::now().timestamp() as u64;
    let jwt = crate::tests::token_support::issuer()
        .issue(&auth_service::tokens::claims::SessionContext {
            account_id: Uuid::new_v4(),
            session_id: Uuid::new_v4(),
            login_at: now,
            expires_at: now + 86_400,
        })
        .unwrap();
    for credential in [
        None,
        Some("wrong"),
        Some("eyJhbGciOiJFZERTQSJ9.payload.signature"),
        Some(jwt.as_str()),
        Some(token(3).as_str()),
    ] {
        assert_problem(
            call(
                &api,
                "/internal/v1/sessions/check",
                credential,
                body.clone(),
            )
            .await,
            StatusCode::UNAUTHORIZED,
            "invalid-service-credentials",
        )
        .await;
    }
    assert_problem(
        call(&api, "/internal/v1/sessions/check", Some(&token(2)), body).await,
        StatusCode::FORBIDDEN,
        "service-forbidden",
    )
    .await;
}

#[tokio::test]
async fn redis_connection_timeout_is_503_without_sql_or_authorization() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (sent, accepted) = tokio::sync::oneshot::channel();
    let stalled = tokio::spawn(async move {
        let (_stream, _) = listener.accept().await.unwrap();
        let _ = sent.send(());
        std::future::pending::<()>().await;
    });
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://test:test@127.0.0.1:1/test")
        .unwrap();
    pool.close().await;
    let credentials =
        ServiceCredentials::parse(&serde_json::to_vec(&document()).unwrap(), Utc::now()).unwrap();
    let api = InternalApi::new(settings(), credentials, format!("redis://{addr}/0"), pool).unwrap();
    let response = timeout(
        Duration::from_secs(1),
        call(
            &api,
            "/internal/v1/sessions/check",
            Some(&token(1)),
            pair(Uuid::new_v4(), Uuid::new_v4()),
        ),
    )
    .await
    .unwrap();
    accepted.await.unwrap();
    assert_problem(
        response,
        StatusCode::SERVICE_UNAVAILABLE,
        "session-check-unavailable",
    )
    .await;
    stalled.abort();
    let _ = stalled.await;
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn http_input_limits_and_duplicate_pairs_never_become_authorization(pool: PgPool) {
    let api = api(pool, settings(), document());
    let one = pair(Uuid::new_v4(), Uuid::new_v4());
    for body in [
        serde_json::json!({"sessions":[]}),
        serde_json::json!({"sessions":[one.clone(),one.clone()]}),
        serde_json::json!({"sessions":[one.clone(),{"sid":"bad","account_id":Uuid::new_v4()}]}),
        serde_json::json!({"sessions":vec![one.clone();101]}),
    ] {
        assert_problem(
            call(
                &api,
                "/internal/v1/sessions/check-batch",
                Some(&token(1)),
                body,
            )
            .await,
            StatusCode::BAD_REQUEST,
            "malformed-request",
        )
        .await;
    }
    for (body, content_type) in [("{", "application/json"), ("{}", "text/plain")] {
        let mut req = request(
            "/internal/v1/sessions/check",
            Some(&token(1)),
            Body::from(body),
        );
        req.headers_mut()
            .insert(header::CONTENT_TYPE, HeaderValue::from_static(content_type));
        assert_problem(
            api.router().oneshot(req).await.unwrap(),
            StatusCode::BAD_REQUEST,
            "malformed-request",
        )
        .await;
    }
    assert_problem(
        api.router()
            .oneshot(request(
                "/internal/v1/sessions/check",
                Some(&token(1)),
                Body::from(" ".repeat(65_537)),
            ))
            .await
            .unwrap(),
        StatusCode::PAYLOAD_TOO_LARGE,
        "request-too-large",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn shared_quota_counts_pairs_and_isolates_core_and_files() {
    let mut config = settings();
    config.pairs_per_second = 1;
    let first = InternalLimits::new(config.clone(), redis_url()).unwrap();
    let mut second = InternalLimits::new(config, redis_url()).unwrap();
    Arc::get_mut(&mut second).unwrap().test_namespace = first.test_namespace.clone();
    first
        .reserve(Identity::Core, 100)
        .await
        .unwrap()
        .release()
        .await
        .unwrap();
    assert!(matches!(
        second.reserve(Identity::Core, 1).await,
        Err(AdmissionError::Limited(_))
    ));
    second
        .reserve(Identity::Files, 100)
        .await
        .unwrap()
        .release()
        .await
        .unwrap();
}

#[tokio::test]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn shared_concurrency_release_and_expired_reservations() {
    let mut config = settings();
    config.concurrent_per_service = 1;
    let first = InternalLimits::new(config.clone(), redis_url()).unwrap();
    let mut second = InternalLimits::new(config, redis_url()).unwrap();
    Arc::get_mut(&mut second).unwrap().test_namespace = first.test_namespace.clone();
    let held = first.reserve(Identity::Core, 1).await.unwrap();
    assert!(matches!(
        second.reserve(Identity::Core, 1).await,
        Err(AdmissionError::Limited(_))
    ));
    second
        .reserve(Identity::Files, 1)
        .await
        .unwrap()
        .release()
        .await
        .unwrap();
    held.release().await.unwrap();
    second
        .reserve(Identity::Core, 1)
        .await
        .unwrap()
        .release()
        .await
        .unwrap();
    // Model a dead replica's expired reservation using Redis fixture state, without sleeping.
    let client = redis::Client::open(redis_url()).unwrap();
    let mut conn = client.get_multiplexed_async_connection().await.unwrap();
    let key = format!("{}:service:{{core}}:leases", first.test_namespace);
    let _: i64 = redis::cmd("ZADD")
        .arg(&key)
        .arg(0)
        .arg("dead-replica")
        .query_async(&mut conn)
        .await
        .unwrap();
    second
        .reserve(Identity::Core, 1)
        .await
        .unwrap()
        .release()
        .await
        .unwrap();
    let expired = first.reserve(Identity::Core, 1).await.unwrap();
    let members: Vec<String> = redis::cmd("ZRANGE")
        .arg(&key)
        .arg(0)
        .arg(-1)
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(members.len(), 1);
    let _: i64 = redis::cmd("ZADD")
        .arg(&key)
        .arg(0)
        .arg(&members[0])
        .query_async(&mut conn)
        .await
        .unwrap();
    assert_eq!(expired.release().await, Err(AdmissionError::Unavailable));
}

#[tokio::test]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn shared_preauth_budget_is_by_socket_ip() {
    let mut config = settings();
    config.preauth_per_second = 1;
    let first = InternalLimits::new(config.clone(), redis_url()).unwrap();
    let mut second = InternalLimits::new(config, redis_url()).unwrap();
    Arc::get_mut(&mut second).unwrap().test_namespace = first.test_namespace.clone();
    first
        .preauthenticate("127.0.0.1".parse().unwrap())
        .await
        .unwrap();
    assert!(matches!(
        second.preauthenticate("127.0.0.1".parse().unwrap()).await,
        Err(AdmissionError::Limited(_))
    ));
    second
        .preauthenticate("127.0.0.2".parse().unwrap())
        .await
        .unwrap();
}

struct PendingBody;
impl hyper::body::Body for PendingBody {
    type Data = Bytes;
    type Error = Infallible;
    fn poll_frame(
        self: Pin<&mut Self>,
        _: &mut Context<'_>,
    ) -> Poll<Option<Result<hyper::body::Frame<Self::Data>, Self::Error>>> {
        Poll::Pending
    }
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn http_dependency_failures_timeouts_and_local_limit_are_fail_closed(pool: PgPool) {
    let body = fixture(&pool).await;
    let mut config = settings();
    config.local_limit = 1;
    let api = api(pool.clone(), config.clone(), document());
    let held = api.local_requests.clone().acquire_owned().await.unwrap();
    assert_problem(
        call(
            &api,
            "/internal/v1/sessions/check",
            Some(&token(1)),
            body.clone(),
        )
        .await,
        StatusCode::TOO_MANY_REQUESTS,
        "service-rate-limited",
    )
    .await;
    drop(held);
    let response = api
        .router()
        .oneshot(request(
            "/internal/v1/sessions/check",
            Some(&token(1)),
            Body::new(PendingBody),
        ))
        .await
        .unwrap();
    assert_problem(
        response,
        StatusCode::SERVICE_UNAVAILABLE,
        "session-check-unavailable",
    )
    .await;
    let mut blocker = pool.begin().await.unwrap();
    sqlx::query("LOCK TABLE auth.sessions IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    assert_problem(
        call(
            &api,
            "/internal/v1/sessions/check",
            Some(&token(1)),
            body.clone(),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
        "session-check-unavailable",
    )
    .await;
    blocker.rollback().await.unwrap();
    pool.close().await;
    assert_problem(
        call(
            &api,
            "/internal/v1/sessions/check",
            Some(&token(1)),
            body.clone(),
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
        "session-check-unavailable",
    )
    .await;
    let credentials =
        ServiceCredentials::parse(&serde_json::to_vec(&document()).unwrap(), Utc::now()).unwrap();
    let closed = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://test:test@127.0.0.1:1/test")
        .unwrap();
    closed.close().await;
    let outage =
        InternalApi::new(config, credentials, "redis://127.0.0.1:1/0".into(), closed).unwrap();
    assert_problem(
        call(
            &outage,
            "/internal/v1/sessions/check",
            Some(&token(1)),
            body,
        )
        .await,
        StatusCode::SERVICE_UNAVAILABLE,
        "session-check-unavailable",
    )
    .await;
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn http_ignores_forged_proxy_headers_and_checks_cidr(pool: PgPool) {
    let api = api(pool, settings(), document());
    let mut req = request(
        "/internal/v1/sessions/check",
        Some(&token(1)),
        Body::from(pair(Uuid::new_v4(), Uuid::new_v4()).to_string()),
    );
    req.extensions_mut().insert(ConnectInfo(
        "192.0.2.5:12345".parse::<SocketAddr>().unwrap(),
    ));
    req.headers_mut()
        .insert("x-real-ip", HeaderValue::from_static("127.0.0.1"));
    assert_problem(
        api.router().oneshot(req).await.unwrap(),
        StatusCode::FORBIDDEN,
        "service-forbidden",
    )
    .await;
}

struct TemporaryFiles(PathBuf);
impl TemporaryFiles {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!("sonimbus-auth-4b-{}", Uuid::new_v4()));
        std::fs::create_dir(&path).unwrap();
        Self(path)
    }
    fn write(&self, name: &str, bytes: impl AsRef<[u8]>) -> PathBuf {
        let path = self.0.join(name);
        std::fs::write(&path, bytes).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        path
    }
    fn certificate(&self, name: &str) -> (PathBuf, PathBuf) {
        let cert = self.0.join(format!("{name}.pem"));
        let key = self.0.join(format!("{name}.key"));
        let status = std::process::Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "ec",
                "-pkeyopt",
                "ec_paramgen_curve:P-256",
                "-nodes",
                "-days",
                "1",
                "-subj",
                "/CN=auth.test",
                "-addext",
                "subjectAltName=DNS:auth.test",
                "-addext",
                "basicConstraints=critical,CA:FALSE",
                "-addext",
                "keyUsage=critical,digitalSignature",
                "-addext",
                "extendedKeyUsage=serverAuth",
            ])
            .arg("-keyout")
            .arg(&key)
            .arg("-out")
            .arg(&cert)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .expect("TLS tests require OpenSSL on PATH");
        assert!(
            status.success(),
            "Cannot generate disposable test certificate"
        );
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600)).unwrap();
        }
        (cert, key)
    }
}
impl Drop for TemporaryFiles {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn real_https_requires_trust_correct_hostname_and_never_accepts_plain_http(pool: PgPool) {
    let files = TemporaryFiles::new();
    let mut config = settings();
    (config.certificate_path, config.key_path) = files.certificate("https");
    let cert_pem = std::fs::read(&config.certificate_path).unwrap();
    let body = fixture(&pool).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    config.addr = addr;
    config.credentials_path =
        files.write("credentials.json", serde_json::to_vec(&document()).unwrap());
    let mut database = reqwest::Url::parse(&std::env::var("DATABASE_URL").unwrap()).unwrap();
    database.set_path(pool.connect_options().get_database().unwrap());
    let server = server::InternalServer::prepare(config, redis_url(), database.as_str())
        .await
        .unwrap();
    let task = tokio::spawn(server.run());
    let root = reqwest::Certificate::from_pem(&cert_pem).unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .add_root_certificate(root)
        .resolve("auth.test", addr)
        .resolve("wrong.test", addr)
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let url = format!(
        "https://auth.test:{}/internal/v1/sessions/check",
        addr.port()
    );
    let response = client
        .post(&url)
        .bearer_auth(token(1))
        .json(&body)
        .send()
        .await
        .unwrap();
    assert_eq!(response.status(), reqwest::StatusCode::OK);
    assert!(
        response.json::<serde_json::Value>().await.unwrap()["active"]
            .as_bool()
            .unwrap()
    );
    let untrusted = reqwest::Client::builder()
        .no_proxy()
        .resolve("auth.test", addr)
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    assert!(untrusted
        .post(&url)
        .bearer_auth(token(1))
        .json(&body)
        .send()
        .await
        .is_err());
    assert!(client
        .post(format!(
            "https://wrong.test:{}/internal/v1/sessions/check",
            addr.port()
        ))
        .json(&body)
        .send()
        .await
        .is_err());
    let mut plaintext = tokio::net::TcpStream::connect(addr).await.unwrap();
    plaintext
        .write_all(b"GET / HTTP/1.1\r\nHost: auth.test\r\n\r\n")
        .await
        .unwrap();
    let mut buffer = [0; 256];
    let received = timeout(Duration::from_secs(1), plaintext.read(&mut buffer))
        .await
        .unwrap();
    // Rejection can close the socket or send a binary TLS alert, never an HTTP response.
    if let Ok(count) = received {
        assert!(!buffer[..count].starts_with(b"HTTP/"));
    }
    // A stalled TLS handshake is also closed within its own budget.
    let mut stalled = tokio::net::TcpStream::connect(addr).await.unwrap();
    let received = timeout(Duration::from_secs(1), stalled.read(&mut buffer))
        .await
        .unwrap();
    assert!(received.is_err() || received.unwrap() == 0);
    task.abort();
    let _ = task.await;
}

#[test]
fn tls_configuration_rejects_missing_invalid_mismatched_and_exposed_keys() {
    let files = TemporaryFiles::new();
    let mut config = settings();
    config.key_path = files.write("private.key", b"not a key");
    config.certificate_path = files.write("certificate.pem", b"not a certificate");
    assert!(server::load_tls(&config).is_err());
    let (first_cert, first_key) = files.certificate("first");
    let (_, second_key) = files.certificate("second");
    config.certificate_path = first_cert;
    config.key_path = second_key;
    assert!(server::load_tls(&config).is_err());
    config.key_path = first_key;
    assert!(server::load_tls(&config).is_ok());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&config.key_path, std::fs::Permissions::from_mode(0o644)).unwrap();
        assert!(server::load_tls(&config).is_err());
    }
}

struct NoMail;
#[async_trait::async_trait]
impl crate::ports::email_verification::VerificationEmailSender for NoMail {
    async fn send(
        &self,
        _: &str,
        _: &str,
        _: &str,
    ) -> Result<(), crate::ports::email_verification::EmailDeliveryError> {
        Ok(())
    }
}

#[tokio::test]
async fn public_router_never_exposes_internal_routes() {
    use crate::{
        repositories::{
            accounts_repository::AccountsRepository,
            one_time_tokens_repository::OneTimeTokensRepository,
        },
        services::{
            account_service::AuthService, email_verification_service::EmailVerificationService,
        },
    };
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://test:test@127.0.0.1:1/test")
        .unwrap();
    pool.close().await;
    let verification = EmailVerificationService::new(
        OneTimeTokensRepository::new(pool.clone()),
        Arc::new(NoMail));
    let public = crate::routes::router(crate::state::AppState {
        login_enabled: false,
        auth_service: AuthService::new(
            AccountsRepository::new(pool.clone()),
            Arc::new(crate::security::password_hasher::Argon2PasswordHasher),
            verification.clone(),
            1,
        ),
        email_verification_service: verification,
        token_issuer: crate::tests::token_support::issuer(),
        auth_rate_limiter: Arc::new(
            crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(
                "redis://127.0.0.1:1/0".into(),
            ),
        ),
        pool,
    });
    for path in [
        "/internal/v1/sessions/check",
        "/internal/v1/sessions/check-batch",
    ] {
        assert_eq!(
            public
                .clone()
                .oneshot(request(path, Some(&token(1)), Body::empty()))
                .await
                .unwrap()
                .status(),
            StatusCode::NOT_FOUND
        );
    }
    for path in ["/health", "/.well-known/jwks.json"] {
        assert_eq!(
            public
                .clone()
                .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
                .await
                .unwrap()
                .status(),
            StatusCode::OK
        );
    }
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn http_reports_service_quota_and_global_concurrency_as_429(pool: PgPool) {
    let mut config = settings();
    config.concurrent_per_service = 1;
    config.pairs_per_second = 1;
    let api = api(pool, config, document());
    let body = pair(Uuid::new_v4(), Uuid::new_v4());
    let held = api.limits.reserve(Identity::Core, 1).await.unwrap();
    assert_problem(
        call(
            &api,
            "/internal/v1/sessions/check",
            Some(&token(1)),
            body.clone(),
        )
        .await,
        StatusCode::TOO_MANY_REQUESTS,
        "service-rate-limited",
    )
    .await;
    held.release().await.unwrap();
    api.limits
        .reserve(Identity::Core, 99)
        .await
        .unwrap()
        .release()
        .await
        .unwrap();
    assert_problem(
        call(
            &api,
            "/internal/v1/sessions/check",
            Some(&token(1)),
            body.clone(),
        )
        .await,
        StatusCode::TOO_MANY_REQUESTS,
        "service-rate-limited",
    )
    .await;
    assert_eq!(
        call(&api, "/internal/v1/sessions/check", Some(&token(2)), body)
            .await
            .status(),
        StatusCode::OK
    );
}

#[derive(Clone)]
struct LogCapture(Arc<std::sync::Mutex<Vec<u8>>>);
impl std::io::Write for LogCapture {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn request_errors_do_not_log_credential_or_digest(pool: PgPool) {
    use tracing::instrument::WithSubscriber;
    let logs = LogCapture(Arc::new(std::sync::Mutex::new(Vec::new())));
    let writer = logs.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    let api = api(pool, settings(), document());
    async {
        assert_problem(
            call(
                &api,
                "/internal/v1/sessions/check",
                Some(&token(3)),
                serde_json::json!({}),
            )
            .await,
            StatusCode::UNAUTHORIZED,
            "invalid-service-credentials",
        )
        .await;
        assert_problem(
            call(
                &api,
                "/internal/v1/sessions/check",
                Some(&token(1)),
                serde_json::json!({"secret":token(1)}),
            )
            .await,
            StatusCode::BAD_REQUEST,
            "malformed-request",
        )
        .await;
    }
    .with_subscriber(subscriber)
    .await;
    let bytes = logs.0.lock().unwrap();
    let output = String::from_utf8_lossy(&bytes);
    for secret in [token(1), token(2), token(3), digest(1), digest(2)] {
        assert!(!output.contains(&secret));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn https_connection_and_header_budgets_are_local_and_bounded(pool: PgPool) {
    use tokio_rustls::rustls::{
        self,
        pki_types::{pem::PemObject, CertificateDer, ServerName},
    };
    let files = TemporaryFiles::new();
    let mut config = settings();
    config.local_limit = 1;
    config.header_timeout = Duration::from_millis(50);
    (config.certificate_path, config.key_path) = files.certificate("limits");
    let cert = std::fs::read(&config.certificate_path).unwrap();
    let acceptor = server::load_tls(&config).unwrap();
    let api = api(pool, config, document());
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(server::InternalServer::for_tests(listener, acceptor, api).run());
    let mut first = tokio::net::TcpStream::connect(addr).await.unwrap();
    let client = reqwest::Client::builder()
        .no_proxy()
        .add_root_certificate(reqwest::Certificate::from_pem(&cert).unwrap())
        .resolve("auth.test", addr)
        .timeout(Duration::from_secs(1))
        .build()
        .unwrap();
    // The first TCP connection occupies the single local TLS slot while handshaking.
    assert!(client
        .post(format!(
            "https://auth.test:{}/internal/v1/sessions/check",
            addr.port()
        ))
        .send()
        .await
        .is_err());
    let mut buffer = [0; 512];
    let received = timeout(Duration::from_secs(1), first.read(&mut buffer))
        .await
        .unwrap();
    assert!(received.is_err() || received.unwrap() == 0);
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(CertificateDer::from_pem_slice(&cert).unwrap())
        .unwrap();
    let client_config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_safe_default_protocol_versions()
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    let connector = tokio_rustls::TlsConnector::from(Arc::new(client_config));
    let stream = tokio::net::TcpStream::connect(addr).await.unwrap();
    let mut tls = connector
        .connect(ServerName::try_from("auth.test").unwrap(), stream)
        .await
        .unwrap();
    tls.write_all(b"POST /internal/v1/sessions/check HTTP/1.1\r\nHost: auth.test\r\n")
        .await
        .unwrap();
    let received = timeout(Duration::from_secs(1), tls.read(&mut buffer))
        .await
        .unwrap();
    if let Ok(count) = received {
        assert!(!String::from_utf8_lossy(&buffer[..count]).contains("200 OK"));
    }
    task.abort();
    let _ = task.await;
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis; use --include-ignored"]
async fn http_rotation_accepts_overlap_and_rejects_expired_or_removed_credential(pool: PgPool) {
    let body = pair(Uuid::new_v4(), Uuid::new_v4());
    let mut doc = document();
    doc["core"]["previous"] = serde_json::json!({"sha256":digest(3),"expires_at":(Utc::now()+Duration::from_secs(60)).to_rfc3339()});
    let overlapping = api(pool.clone(), settings(), doc.clone());
    for seed in [1, 3] {
        assert_eq!(
            call(
                &overlapping,
                "/internal/v1/sessions/check",
                Some(&token(seed)),
                body.clone()
            )
            .await
            .status(),
            StatusCode::OK
        );
    }
    doc["core"]["previous"]["expires_at"] =
        (Utc::now() - Duration::from_secs(1)).to_rfc3339().into();
    let expired = api(pool.clone(), settings(), doc.clone());
    assert_problem(
        call(
            &expired,
            "/internal/v1/sessions/check",
            Some(&token(3)),
            body.clone(),
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "invalid-service-credentials",
    )
    .await;
    doc["core"].as_object_mut().unwrap().remove("previous");
    let removed = api(pool, settings(), doc);
    assert_problem(
        call(
            &removed,
            "/internal/v1/sessions/check",
            Some(&token(3)),
            body,
        )
        .await,
        StatusCode::UNAUTHORIZED,
        "invalid-service-credentials",
    )
    .await;
}
