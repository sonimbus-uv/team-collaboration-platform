//! sqlx::test creates isolated databases. No user schemas or data are reused.
use crate::{
    dto::sessions::{ClientType, LoginRequest, TokenResponse},
    ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
    repositories::{
        accounts_repository::AccountsRepository,
        registration_attempts_repository::RegistrationAttemptsRepository,
        sessions_repository::SessionsRepository,
    },
    security::password_hasher::{PasswordHasher, PasswordHashingError},
    services::{
        account_service::AuthService,
        email_verification_service::EmailVerificationService,
        session_service::{refresh_hash, AccessSigner, SessionError, SessionService},
    },
    state::AppState,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;
use uuid::Uuid;

struct NoEmail;
#[async_trait::async_trait]
impl VerificationEmailSender for NoEmail {
    async fn send(&self, _: &str, _: &str, _: &str) -> Result<(), EmailDeliveryError> {
        Ok(())
    }
}
struct TestHasher;
impl PasswordHasher for TestHasher {
    fn hash_password(&self, _: &str) -> Result<String, PasswordHashingError> {
        Ok("test-only-password-hash".into())
    }
    fn verify_password(&self, password: &str, hash: &str) -> Result<bool, PasswordHashingError> {
        Ok(password == "TestPass123!" && hash == "test-only-password-hash")
    }
}
fn auth(pool: &PgPool, hasher: Arc<dyn PasswordHasher>) -> AuthService {
    AuthService::new(
        AccountsRepository::new(pool.clone()),
        hasher,
        EmailVerificationService::new(
            RegistrationAttemptsRepository::new(pool.clone()),
            Arc::new(NoEmail),
            &[0x17; 32],
        )
        .unwrap(),
        4,
    )
}
fn service(pool: &PgPool) -> SessionService {
    SessionService::new(
        SessionsRepository::new(pool.clone()),
        auth(pool, Arc::new(TestHasher)),
        crate::tests::token_support::issuer(),
    )
}
async fn account(pool: &PgPool) -> Uuid {
    let id:Uuid=sqlx::query_scalar("INSERT INTO auth.accounts(email,email_verified_at) VALUES('person@example.invalid',clock_timestamp()) RETURNING id").fetch_one(pool).await.unwrap();
    sqlx::query("INSERT INTO auth.password_credentials(account_id,password_hash) VALUES($1,'test-only-password-hash')").bind(id).execute(pool).await.unwrap();
    id
}
fn request(client: ClientType) -> LoginRequest {
    LoginRequest {
        email: " PERSON@Example.Invalid ".into(),
        password: "TestPass123!".into(),
        client_type: client,
        device_name: Some("test-only-device".into()),
    }
}
async fn active(pool: &PgPool, id: Uuid) -> bool {
    sqlx::query_scalar("SELECT revoked_at IS NULL FROM auth.sessions WHERE id=$1")
        .bind(id)
        .fetch_one(pool)
        .await
        .unwrap()
}
async fn events(pool: &PgPool) -> i64 {
    sqlx::query_scalar("SELECT count(*) FROM auth.session_revocation_outbox")
        .fetch_one(pool)
        .await
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn login_limits_independence_replacement_and_outbox(pool: PgPool) {
    let id = account(&pool).await;
    let service = service(&pool);
    let first = service.login(request(ClientType::Desktop)).await.unwrap();
    assert_eq!(first.account_id, id);
    assert_eq!(first.access_expires_in, 900);
    let lifetime: i64 = sqlx::query_scalar(
        "SELECT extract(epoch FROM expires_at-created_at)::bigint FROM auth.sessions WHERE id=$1",
    )
    .bind(first.sid)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(lifetime, 20 * 86400);
    let mobile = service.login(request(ClientType::Mobile)).await.unwrap();
    let next = service.login(request(ClientType::Desktop)).await.unwrap();
    assert!(!active(&pool, first.sid).await);
    assert!(active(&pool, mobile.sid).await && active(&pool, next.sid).await);
    assert_eq!(events(&pool).await, 1);
    let event: serde_json::Value =
        sqlx::query_scalar("SELECT event FROM auth.session_revocation_outbox")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(event["payload"]["reason"], "superseded");
    assert_eq!(event["payload"]["sid"], first.sid.to_string());
    let hashes: Vec<Vec<u8>> = sqlx::query_scalar("SELECT token_hash FROM auth.refresh_tokens")
        .fetch_all(&pool)
        .await
        .unwrap();
    assert!(hashes.contains(&refresh_hash(&next.refresh_token).unwrap().to_vec()));
    assert!(hashes.iter().all(|h| h.len() == 32));
}

#[sqlx::test(migrations = "./migrations")]
async fn wrong_credentials_and_account_states_are_generic(pool: PgPool) {
    let id = account(&pool).await;
    let service = service(&pool);
    let mut wrong = request(ClientType::Desktop);
    wrong.password = "wrong".into();
    assert!(matches!(
        service.login(wrong).await,
        Err(SessionError::InvalidCredentials)
    ));
    let mut absent = request(ClientType::Desktop);
    absent.email = "absent@example.invalid".into();
    assert!(matches!(
        service.login(absent).await,
        Err(SessionError::InvalidCredentials)
    ));
    for status in ["locked", "suspended", "deleted"] {
        sqlx::query("UPDATE auth.accounts SET status=$2 WHERE id=$1")
            .bind(id)
            .bind(status)
            .execute(&pool)
            .await
            .unwrap();
        assert!(matches!(
            service.login(request(ClientType::Desktop)).await,
            Err(SessionError::InvalidCredentials)
        ));
    }
    sqlx::query("UPDATE auth.accounts SET status='active',email_verified_at=NULL WHERE id=$1")
        .bind(id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        service.login(request(ClientType::Desktop)).await,
        Err(SessionError::InvalidCredentials)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn rotation_preserves_expiry_and_replay_commits_revocation(pool: PgPool) {
    account(&pool).await;
    let service = service(&pool);
    let first = service.login(request(ClientType::Desktop)).await.unwrap();
    let second = service.refresh(&first.refresh_token).await.unwrap();
    assert_eq!(first.session_expires_at, second.session_expires_at);
    assert_ne!(first.refresh_token, second.refresh_token);
    assert_eq!(first.sid, second.sid);
    assert!(matches!(
        service.refresh(&first.refresh_token).await,
        Err(SessionError::InvalidRefresh)
    ));
    assert!(!active(&pool, first.sid).await);
    assert_eq!(events(&pool).await, 1);
    assert!(matches!(
        service.refresh(&second.refresh_token).await,
        Err(SessionError::InvalidRefresh)
    ));
    let live: i64 =
        sqlx::query_scalar("SELECT count(*) FROM auth.refresh_tokens WHERE revoked_at IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(live, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_refresh_revokes_family(pool: PgPool) {
    account(&pool).await;
    let service = service(&pool);
    let first = service.login(request(ClientType::Desktop)).await.unwrap();
    let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            service.refresh(&first.refresh_token),
            service.refresh(&first.refresh_token)
        )
    })
    .await
    .unwrap();
    assert_eq!(usize::from(a.is_ok()) + usize::from(b.is_ok()), 1);
    assert!(!active(&pool, first.sid).await);
    assert_eq!(events(&pool).await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_logins_last_wins(pool: PgPool) {
    account(&pool).await;
    let service = service(&pool);
    let (a, b) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            service.login(request(ClientType::Desktop)),
            service.login(request(ClientType::Desktop))
        )
    })
    .await
    .unwrap();
    let a = a.unwrap();
    let b = b.unwrap();
    assert_ne!(a.sid, b.sid);
    assert_ne!(active(&pool, a.sid).await, active(&pool, b.sid).await);
    assert_eq!(events(&pool).await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn logout_is_idempotent_and_does_not_reactivate_or_affect_mobile(pool: PgPool) {
    account(&pool).await;
    let service = service(&pool);
    let first = service.login(request(ClientType::Desktop)).await.unwrap();
    let mobile = service.login(request(ClientType::Mobile)).await.unwrap();
    let rotated = service.refresh(&first.refresh_token).await.unwrap();
    // Historical consumed token can identify its own family for logout.
    service.logout(&first.refresh_token).await.unwrap();
    service.logout(&rotated.refresh_token).await.unwrap();
    service.logout("unknown").await.unwrap();
    assert_eq!(events(&pool).await, 1);
    assert!(active(&pool, mobile.sid).await);
    assert!(matches!(
        service.refresh(&rotated.refresh_token).await,
        Err(SessionError::InvalidRefresh)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_racing_logout_cannot_resurrect(pool: PgPool) {
    account(&pool).await;
    let service = service(&pool);
    let first = service.login(request(ClientType::Desktop)).await.unwrap();
    let (result, logout) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            service.refresh(&first.refresh_token),
            service.logout(&first.refresh_token)
        )
    })
    .await
    .unwrap();
    logout.unwrap();
    assert!(!active(&pool, first.sid).await);
    if let Ok(next) = result {
        assert!(service.refresh(&next.refresh_token).await.is_err());
    }
    assert_eq!(events(&pool).await, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_racing_replacement_cannot_resurrect(pool: PgPool) {
    account(&pool).await;
    let service = service(&pool);
    let first = service.login(request(ClientType::Desktop)).await.unwrap();
    let (result, next) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
        tokio::join!(
            service.refresh(&first.refresh_token),
            service.login(request(ClientType::Desktop))
        )
    })
    .await
    .unwrap();
    let next = next.unwrap();
    assert!(active(&pool, next.sid).await);
    assert!(!active(&pool, first.sid).await);
    if let Ok(rotated) = result {
        assert!(service.refresh(&rotated.refresh_token).await.is_err());
    }
}

struct FailingSigner;
impl AccessSigner for FailingSigner {
    fn sign(
        &self,
        _: &auth_service::tokens::claims::SessionContext,
    ) -> Result<(String, u64), SessionError> {
        Err(SessionError::Signing)
    }
}

struct VerificationGate {
    entered: std::sync::Mutex<Option<tokio::sync::oneshot::Sender<()>>>,
    release: std::sync::Mutex<std::sync::mpsc::Receiver<()>>,
}
impl PasswordHasher for VerificationGate {
    fn hash_password(&self, _: &str) -> Result<String, PasswordHashingError> {
        unreachable!()
    }
    fn verify_password(&self, _: &str, _: &str) -> Result<bool, PasswordHashingError> {
        self.entered
            .lock()
            .unwrap()
            .take()
            .unwrap()
            .send(())
            .unwrap();
        self.release
            .lock()
            .unwrap()
            .recv_timeout(std::time::Duration::from_secs(10))
            .unwrap();
        Ok(true)
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn credential_and_account_changes_after_verification_are_rechecked(pool: PgPool) {
    let id = account(&pool).await;
    for change in [
        "UPDATE auth.password_credentials SET password_hash='changed' WHERE account_id=$1",
        "UPDATE auth.accounts SET status='suspended' WHERE id=$1",
    ] {
        sqlx::query("UPDATE auth.password_credentials SET password_hash='test-only-password-hash' WHERE account_id=$1").bind(id).execute(&pool).await.unwrap();
        sqlx::query("UPDATE auth.accounts SET status='active' WHERE id=$1")
            .bind(id)
            .execute(&pool)
            .await
            .unwrap();
        let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let passwords = auth(
            &pool,
            Arc::new(VerificationGate {
                entered: std::sync::Mutex::new(Some(entered_tx)),
                release: std::sync::Mutex::new(release_rx),
            }),
        );
        let service = SessionService::new(
            SessionsRepository::new(pool.clone()),
            passwords,
            crate::tests::token_support::issuer(),
        );
        let pending =
            tokio::spawn(async move { service.login(request(ClientType::Desktop)).await });
        tokio::time::timeout(std::time::Duration::from_secs(10), entered_rx)
            .await
            .unwrap()
            .unwrap();
        sqlx::query(change).bind(id).execute(&pool).await.unwrap();
        release_tx.send(()).unwrap();
        assert!(matches!(
            pending.await.unwrap(),
            Err(SessionError::InvalidCredentials)
        ));
    }
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn bounded_password_verification_runs_off_runtime_and_shares_register_budget(pool: PgPool) {
    let (entered_tx, entered_rx) = tokio::sync::oneshot::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let passwords = AuthService::new(
        AccountsRepository::new(pool.clone()),
        Arc::new(VerificationGate {
            entered: std::sync::Mutex::new(Some(entered_tx)),
            release: std::sync::Mutex::new(release_rx),
        }),
        EmailVerificationService::new(
            RegistrationAttemptsRepository::new(pool.clone()),
            Arc::new(NoEmail),
            &[0x17; 32],
        )
        .unwrap(),
        1,
    );
    let copy = passwords.clone();
    let pending = tokio::spawn(async move {
        copy.check_password("test-only".into(), Some("test-only-hash".into()))
            .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(10), entered_rx)
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(
        passwords
            .check_password("test-only".into(), Some("test-only-hash".into()))
            .await,
        Err(crate::services::account_service::PasswordCheckError::Busy)
    ));
    assert!(matches!(
        passwords
            .register(
                "other@example.invalid".into(),
                "TestPass123!".into(),
                &crate::ports::email_verification::TestBudget
            )
            .await,
        Err(crate::services::account_service::RegisterError::Busy)
    ));
    release_tx.send(()).unwrap();
    assert!(pending.await.unwrap().unwrap());
}
#[sqlx::test(migrations = "./migrations")]
async fn signing_failure_rolls_back_open_rotation_and_outbox(pool: PgPool) {
    account(&pool).await;
    let good = service(&pool);
    let first = good.login(request(ClientType::Desktop)).await.unwrap();
    let failing = SessionService::new(
        SessionsRepository::new(pool.clone()),
        auth(&pool, Arc::new(TestHasher)),
        Arc::new(FailingSigner),
    );
    assert!(matches!(
        failing.login(request(ClientType::Desktop)).await,
        Err(SessionError::Signing)
    ));
    assert!(active(&pool, first.sid).await);
    assert_eq!(events(&pool).await, 0);
    assert!(matches!(
        failing.refresh(&first.refresh_token).await,
        Err(SessionError::Signing)
    ));
    good.refresh(&first.refresh_token).await.unwrap();
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.refresh_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn outbox_failure_rolls_back_revocation_and_database_failure_rolls_back_open(pool: PgPool) {
    account(&pool).await;
    let service = service(&pool);
    let first = service.login(request(ClientType::Desktop)).await.unwrap();
    sqlx::query("ALTER TABLE auth.session_revocation_outbox ADD CONSTRAINT fail_event CHECK(false) NOT VALID").execute(&pool).await.unwrap();
    assert!(service.logout(&first.refresh_token).await.is_err());
    assert!(active(&pool, first.sid).await);
    assert!(service.login(request(ClientType::Desktop)).await.is_err());
    assert!(active(&pool, first.sid).await);
    assert_eq!(events(&pool).await, 0);
    sqlx::query("ALTER TABLE auth.session_revocation_outbox DROP CONSTRAINT fail_event")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "ALTER TABLE auth.refresh_tokens ADD CONSTRAINT fail_refresh CHECK(false) NOT VALID",
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(service.login(request(ClientType::Desktop)).await.is_err());
    assert!(active(&pool, first.sid).await);
    assert_eq!(events(&pool).await, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn expiration_is_checked_without_waiting_and_lifetime_cannot_extend(pool: PgPool) {
    let id = account(&pool).await;
    let service = service(&pool);
    let first = service.login(request(ClientType::Desktop)).await.unwrap();
    sqlx::query("UPDATE auth.refresh_tokens SET expires_at=clock_timestamp()-interval '1 second'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(service.refresh(&first.refresh_token).await.is_err());
    assert!(sqlx::query(
        "UPDATE auth.sessions SET expires_at=expires_at+interval '1 second' WHERE id=$1"
    )
    .bind(first.sid)
    .execute(&pool)
    .await
    .is_err());
    service.logout(&first.refresh_token).await.unwrap();
    assert!(
        sqlx::query("UPDATE auth.sessions SET revoked_at=NULL,revoke_reason=NULL WHERE id=$1")
            .bind(first.sid)
            .execute(&pool)
            .await
            .is_err()
    );
    assert!(
        sqlx::query("SELECT auth.open_session($1,'desktop',interval '21 days')")
            .bind(id)
            .execute(&pool)
            .await
            .is_err()
    );
    // A preexpired fixture, preserving the 20-day invariant, requires no sleeps.
    let sid:Uuid=sqlx::query_scalar("INSERT INTO auth.sessions(account_id,client_type,created_at,expires_at) VALUES($1,'desktop',clock_timestamp()-interval '20 days',clock_timestamp()-interval '1 second') RETURNING id").bind(id).fetch_one(&pool).await.unwrap();
    let secret = base64::Engine::encode(
        &base64::engine::general_purpose::URL_SAFE_NO_PAD,
        [42u8; 32],
    );
    sqlx::query("INSERT INTO auth.refresh_tokens(session_id,token_hash,expires_at) VALUES($1,$2,clock_timestamp()+interval '1 day')").bind(sid).bind(refresh_hash(&secret).unwrap().to_vec()).execute(&pool).await.unwrap();
    assert!(matches!(
        service.refresh(&secret).await,
        Err(SessionError::InvalidRefresh)
    ));
}

fn app(pool: &PgPool, enabled: bool) -> axum::Router {
    crate::routes::router(AppState {
        login_enabled: enabled,
        pool: pool.clone(),
        auth_service: auth(pool, Arc::new(TestHasher)),
        email_verification_service: EmailVerificationService::new(
            RegistrationAttemptsRepository::new(pool.clone()),
            Arc::new(NoEmail),
            &[0x17; 32],
        )
        .unwrap(),
        token_issuer: crate::tests::token_support::issuer(),
        auth_rate_limiter: Arc::new(
            crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(
                "redis://127.0.0.1:1/0".into(),
            )
            .without_redis_for_tests(),
        ),
    })
}

fn protected_app(
    pool: &PgPool,
    limiter: crate::protection::auth_rate_limiter::AuthRateLimiter,
) -> axum::Router {
    crate::routes::router(AppState {
        login_enabled: true,
        pool: pool.clone(),
        auth_service: auth(pool, Arc::new(TestHasher)),
        email_verification_service: EmailVerificationService::new(
            RegistrationAttemptsRepository::new(pool.clone()),
            Arc::new(NoEmail),
            &[0x17; 32],
        )
        .unwrap(),
        token_issuer: crate::tests::token_support::issuer(),
        auth_rate_limiter: Arc::new(limiter),
    })
}

#[sqlx::test(migrations = "./migrations")]
async fn redis_outage_rejects_session_http_without_consuming_refresh(pool: PgPool) {
    account(&pool).await;
    let service = service(&pool);
    let tokens = service.login(request(ClientType::Desktop)).await.unwrap();
    let router = protected_app(
        &pool,
        crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(
            "redis://127.0.0.1:1/0".into(),
        ),
    );
    let (status, headers, body) = http(
        router,
        "/api/v1/auth/refresh",
        serde_json::json!({"refresh_token":tokens.refresh_token}),
    )
    .await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(body["type"], "/problems/auth-unavailable");
    assert!(active(&pool, tokens.sid).await);
    service.refresh(&tokens.refresh_token).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires Redis; CI runs --include-ignored"]
async fn redis_login_account_budget_and_refresh_ip_budget_are_independent(pool: PgPool) {
    let id = account(&pool).await;
    let email = format!("rate-{}@example.invalid", Uuid::new_v4());
    sqlx::query("UPDATE auth.accounts SET email=$2 WHERE id=$1")
        .bind(id)
        .bind(&email)
        .execute(&pool)
        .await
        .unwrap();
    let redis = std::env::var("REDIS_URL").expect("Redis test requires REDIS_URL");
    let router = protected_app(
        &pool,
        crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(redis.clone())
            .with_login_account_policy_for_tests(
                crate::protection::rate_limit::BucketPolicy::new(1, 60).unwrap(),
            ),
    );
    let body = serde_json::json!({"email":email,"password":"TestPass123!","client_type":"desktop"});
    let (status, _, tokens) = http(router.clone(), "/api/v1/auth/login", body.clone()).await;
    assert_eq!(status, StatusCode::OK);
    let (status, headers, problem) = http(router, "/api/v1/auth/login", body).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert!(headers.contains_key("retry-after"));
    assert_eq!(problem["type"], "/problems/too-many-requests");
    let router = protected_app(
        &pool,
        crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(redis)
            .with_ip_policy_for_tests(
                vec![],
                crate::protection::rate_limit::BucketPolicy::new(1, 60).unwrap(),
            ),
    );
    let body = serde_json::json!({"refresh_token":tokens["refresh_token"]});
    let (status, _, next) = http(router.clone(), "/api/v1/auth/refresh", body.clone()).await;
    assert_eq!(status, StatusCode::OK);
    let (status, _, problem) = http(router, "/api/v1/auth/refresh", body).await;
    assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(problem["type"], "/problems/too-many-requests");
    // The second request was rejected before replay detection; the rotated session remains usable.
    assert!(
        active(
            &pool,
            Uuid::parse_str(next["sid"].as_str().unwrap()).unwrap()
        )
        .await
    );
}
async fn http(
    app: axum::Router,
    path: &str,
    body: serde_json::Value,
) -> (StatusCode, axum::http::HeaderMap, serde_json::Value) {
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(path)
                .header("content-type", "application/json")
                .extension(axum::extract::ConnectInfo(
                    "192.0.2.1:45000".parse::<std::net::SocketAddr>().unwrap(),
                ))
                .body(Body::from(serde_json::to_vec(&body).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    let body = if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, headers, body)
}
#[sqlx::test(migrations = "./migrations")]
async fn http_contract_headers_gate_and_redaction(pool: PgPool) {
    account(&pool).await;
    let login = serde_json::json!({"email":"person@example.invalid","password":"TestPass123!","client_type":"desktop"});
    let (status, _, body) = http(app(&pool, false), "/api/v1/auth/login", login.clone()).await;
    assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["type"], "/problems/auth-unavailable");
    let router = app(&pool, true);
    let (status, headers, tokens) = http(router.clone(), "/api/v1/auth/login", login).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers["cache-control"], "no-store");
    assert_eq!(tokens["token_type"], "Bearer");
    let (status, _, next) = http(
        router.clone(),
        "/api/v1/auth/refresh",
        serde_json::json!({"refresh_token":tokens["refresh_token"]}),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    let logout = serde_json::json!({"refresh_token":next["refresh_token"]});
    assert_eq!(
        http(router.clone(), "/api/v1/auth/logout", logout.clone())
            .await
            .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        http(router.clone(), "/api/v1/auth/logout", logout).await.0,
        StatusCode::NO_CONTENT
    );
    for body in [
        serde_json::json!({}),
        serde_json::json!({"refresh_token":""}),
        serde_json::json!({"refresh_token":"x","extra":"secret"}),
    ] {
        let (status, headers, problem) = http(router.clone(), "/api/v1/auth/refresh", body).await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(headers["cache-control"], "no-store");
        assert_eq!(problem["type"], "/problems/malformed-request");
    }
    let (status, _, problem) = http(
        router,
        "/api/v1/auth/refresh",
        serde_json::json!({"refresh_token":"unknown"}),
    )
    .await;
    assert_eq!(status, StatusCode::UNAUTHORIZED);
    assert_eq!(problem["type"], "/problems/invalid-refresh-token");
    assert!(!format!("{:?}", request(ClientType::Desktop)).contains("TestPass123!"));
    let response = TokenResponse {
        access_token: "access-secret".into(),
        refresh_token: "refresh-secret".into(),
        token_type: "Bearer",
        access_expires_in: 900,
        session_expires_at: "unused".into(),
        sid: Uuid::new_v4(),
        account_id: Uuid::new_v4(),
    };
    assert!(!format!("{response:?}").contains("secret"));
}
