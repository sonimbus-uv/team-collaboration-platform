//! Each test uses an isolated sqlx database; codes only exist in this in-memory mail fixture.
use crate::{
    ports::email_verification::{
        EmailDeliveryError, RecipientBudget, TestBudget, VerificationEmailSender,
    },
    repositories::{
        accounts_repository::AccountsRepository,
        registration_attempts_repository::RegistrationAttemptsRepository,
    },
    security::password_hasher::{Argon2PasswordHasher, PasswordHasher},
    services::{
        account_service::AuthService,
        email_verification_service::{EmailVerificationService, VerificationError},
    },
    state::AppState,
};
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Default)]
struct Mail {
    messages: Mutex<Vec<String>>,
    fail: AtomicBool,
}
#[async_trait::async_trait]
impl VerificationEmailSender for Mail {
    async fn send(&self, _: &str, code: &str, _: &str) -> Result<(), EmailDeliveryError> {
        if self.fail.load(Ordering::SeqCst) {
            return Err(EmailDeliveryError::Transport);
        }
        self.messages.lock().unwrap().push(code.into());
        Ok(())
    }
}
impl Mail {
    fn last(&self) -> String {
        self.messages.lock().unwrap().last().unwrap().clone()
    }
}
fn state(pool: &PgPool, mail: Arc<Mail>) -> AppState {
    let verification = EmailVerificationService::new(
        RegistrationAttemptsRepository::new(pool.clone()),
        mail,
        &[0x17; 32],
    )
    .unwrap();
    AppState {
        login_enabled: true, // Isolated test only; production default remains false.
        token_issuer: crate::tests::token_support::issuer(),
        pool: pool.clone(),
        auth_service: AuthService::new(
            AccountsRepository::new(pool.clone()),
            Arc::new(Argon2PasswordHasher),
            verification.clone(),
            4,
        ),
        email_verification_service: verification,
        auth_rate_limiter: Arc::new(
            crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(
                "redis://127.0.0.1:1/0".into(),
            )
            .without_redis_for_tests(),
        ),
    }
}
fn request(path: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .extension(axum::extract::ConnectInfo(
            "192.0.2.17:4017".parse::<std::net::SocketAddr>().unwrap(),
        ))
        .body(Body::from(body.to_string()))
        .unwrap()
}
async fn call(
    app: &axum::Router,
    path: &str,
    body: serde_json::Value,
    status: StatusCode,
) -> serde_json::Value {
    let response = app.clone().oneshot(request(path, body)).await.unwrap();
    assert_eq!(response.status(), status);
    assert_eq!(response.headers()["cache-control"], "no-store");
    let bytes = to_bytes(response.into_body(), 16384).await.unwrap();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}
async fn start(state: &AppState, email: &str, password: &str) -> Uuid {
    state
        .auth_service
        .register(email.into(), password.into(), &TestBudget)
        .await
        .unwrap()
        .0
        .id
}
async fn age(pool: &PgPool, id: Uuid) {
    sqlx::query("UPDATE auth.registration_attempts SET last_sent_at=clock_timestamp()-interval '61 seconds' WHERE id=$1").bind(id).execute(pool).await.unwrap();
}
async fn expire(pool: &PgPool, id: Uuid) {
    sqlx::query("UPDATE auth.registration_attempts SET created_at=clock_timestamp()-interval '11 minutes',expires_at=clock_timestamp()-interval '1 second' WHERE id=$1").bind(id).execute(pool).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn risk17_owner_attempt_wins_and_full_session_lifecycle(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    let state = state(&pool, mail.clone());
    let app = crate::routes::router(state.clone());
    let attacker = start(&state, "owner@example.com", "AttackerPass123!").await;
    let code_a = mail.last();
    let result = call(
        &app,
        "/auth/register",
        serde_json::json!({"email":" OWNER@Example.COM ","password":"OwnerPass456!"}),
        StatusCode::CREATED,
    )
    .await;
    let owner: Uuid = result["registration_attempt_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let code_b = mail.last();
    assert!(result.get("user_id").is_none());
    assert!(result.get("code").is_none());
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth.accounts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    call(
        &app,
        "/auth/verify-email",
        serde_json::json!({"registration_attempt_id":attacker,"code":code_b}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    call(
        &app,
        "/auth/verify-email",
        serde_json::json!({"registration_attempt_id":owner,"code":code_b}),
        StatusCode::OK,
    )
    .await;
    call(
        &app,
        "/auth/verify-email",
        serde_json::json!({"registration_attempt_id":attacker,"code":code_a}),
        StatusCode::BAD_REQUEST,
    )
    .await;
    let login = |password: &str| serde_json::json!({"email":"owner@example.com","password":password,"client_type":"desktop"});
    call(
        &app,
        "/api/v1/auth/login",
        login("AttackerPass123!"),
        StatusCode::UNAUTHORIZED,
    )
    .await;
    let tokens = call(
        &app,
        "/api/v1/auth/login",
        login("OwnerPass456!"),
        StatusCode::OK,
    )
    .await;
    assert!(tokens["access_token"].as_str().unwrap().split('.').count() == 3);
    let checks = auth_service::session_check_service::SessionCheckService::new(pool.clone());
    let pair = auth_service::session_check_types::SessionCheckRequest {
        sid: tokens["sid"].as_str().unwrap().parse().unwrap(),
        account_id: tokens["account_id"].as_str().unwrap().parse().unwrap(),
    };
    assert!(checks.check(pair).await.unwrap().active);
    let next = call(
        &app,
        "/api/v1/auth/refresh",
        serde_json::json!({"refresh_token":tokens["refresh_token"]}),
        StatusCode::OK,
    )
    .await;
    assert_ne!(tokens["refresh_token"], next["refresh_token"]);
    call(
        &app,
        "/api/v1/auth/logout",
        serde_json::json!({"refresh_token":next["refresh_token"]}),
        StatusCode::NO_CONTENT,
    )
    .await;
    assert!(!checks.check(pair).await.unwrap().active);
    call(
        &app,
        "/api/v1/auth/refresh",
        serde_json::json!({"refresh_token":next["refresh_token"]}),
        StatusCode::UNAUTHORIZED,
    )
    .await;
}

#[sqlx::test(migrations = "./migrations")]
async fn wrong_codes_persist_and_fifth_failure_locks_even_correct_code(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    let state = state(&pool, mail.clone());
    let app = crate::routes::router(state.clone());
    let id = start(&state, "failures@example.com", "ValidPass123!").await;
    let code = mail.last();
    for expected in 1..=5 {
        call(
            &app,
            "/auth/verify-email",
            serde_json::json!({"registration_attempt_id":id,"code":"invalid-format"}),
            StatusCode::BAD_REQUEST,
        )
        .await;
        let count: i32 = sqlx::query_scalar(
            "SELECT failed_attempts FROM auth.registration_attempts WHERE id=$1",
        )
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
        assert_eq!(count, expected);
    }
    assert!(matches!(
        state.email_verification_service.confirm(id, &code).await,
        Err(VerificationError::Invalid)
    ));
    age(&pool, id).await;
    assert!(matches!(
        state
            .email_verification_service
            .resend(id, &TestBudget)
            .await,
        Err(VerificationError::Invalid)
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth.accounts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn expired_consumed_and_unknown_attempts_are_unusable(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    let state = state(&pool, mail.clone());
    let id = start(&state, "expired@example.com", "ValidPass123!").await;
    let code = mail.last();
    expire(&pool, id).await;
    assert!(matches!(
        state.email_verification_service.confirm(id, &code).await,
        Err(VerificationError::Invalid)
    ));
    assert!(matches!(
        state
            .email_verification_service
            .confirm(Uuid::new_v4(), &code)
            .await,
        Err(VerificationError::Invalid)
    ));
    let next = start(&state, "expired@example.com", "ValidPass456!").await;
    let code = mail.last();
    state
        .email_verification_service
        .confirm(next, &code)
        .await
        .unwrap();
    assert!(matches!(
        state.email_verification_service.confirm(next, &code).await,
        Err(VerificationError::Invalid)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn resends_replace_code_preserve_failures_deadline_and_have_fixed_limits(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    let state = state(&pool, mail.clone());
    let id = start(&state, "resend@example.com", "ValidPass123!").await;
    let old = mail.last();
    state
        .email_verification_service
        .confirm(id, "wrong")
        .await
        .unwrap_err();
    assert!(matches!(
        state
            .email_verification_service
            .resend(id, &TestBudget)
            .await,
        Err(VerificationError::Limited(_))
    ));
    let expiry = sqlx::query_scalar::<_, sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>>(
        "SELECT expires_at FROM auth.registration_attempts WHERE id=$1",
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    for _ in 0..3 {
        age(&pool, id).await;
        state
            .email_verification_service
            .resend(id, &TestBudget)
            .await
            .unwrap();
    }
    let (count,version,actual):(i32,i32,sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>)=sqlx::query_as("SELECT failed_attempts,code_version,expires_at FROM auth.registration_attempts WHERE id=$1").bind(id).fetch_one(&pool).await.unwrap();
    assert_eq!((count, version), (1, 3));
    assert_eq!(actual, expiry);
    age(&pool, id).await;
    assert!(matches!(
        state
            .email_verification_service
            .resend(id, &TestBudget)
            .await,
        Err(VerificationError::Limited(_))
    ));
    assert!(state
        .email_verification_service
        .confirm(id, &old)
        .await
        .is_err());
    state
        .email_verification_service
        .confirm(id, &mail.last())
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn new_attempts_never_evict_existing_and_pending_limit_is_bounded(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    let state = state(&pool, mail.clone());
    let owner = start(&state, "pending@example.com", "OwnerPass456!").await;
    let code = mail.last();
    for _ in 0..4 {
        start(&state, "pending@example.com", "AttackerPass123!").await;
    }
    assert!(matches!(
        state
            .auth_service
            .register(
                "pending@example.com".into(),
                "AttackerPass123!".into(),
                &TestBudget
            )
            .await,
        Err(
            crate::services::account_service::RegisterError::Verification(
                VerificationError::Limited(_)
            )
        )
    ));
    state
        .email_verification_service
        .confirm(owner, &code)
        .await
        .unwrap();
    let hash: String = sqlx::query_scalar("SELECT password_hash FROM auth.password_credentials")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(Argon2PasswordHasher
        .verify_password("OwnerPass456!", &hash)
        .unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_same_attempt_and_distinct_attempts_have_only_one_winner(pool: PgPool) {
    for same in [true, false] {
        let mail = Arc::new(Mail::default());
        let state = state(&pool, mail.clone());
        let email = format!("race-{same}@example.com");
        let a = start(&state, &email, "FirstPass123!").await;
        let ca = mail.last();
        let b = if same {
            a
        } else {
            start(&state, &email, "OtherPass456!").await
        };
        let cb = mail.last();
        let (one, two) = tokio::join!(
            state.email_verification_service.confirm(a, &ca),
            state.email_verification_service.confirm(b, &cb)
        );
        assert_eq!(usize::from(one.is_ok()) + usize::from(two.is_ok()), 1);
        let hash:String=sqlx::query_scalar("SELECT p.password_hash FROM auth.password_credentials p JOIN auth.accounts a ON a.id=p.account_id WHERE a.email=$1").bind(&email).fetch_one(&pool).await.unwrap();
        let password = if one.is_ok() || same {
            "FirstPass123!"
        } else {
            "OtherPass456!"
        };
        assert!(Argon2PasswordHasher
            .verify_password(password, &hash)
            .unwrap());
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn verified_accounts_cannot_be_changed_and_legacy_pending_can_be_claimed(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    let state = state(&pool, mail.clone());
    let account: Uuid = sqlx::query_scalar(
        "INSERT INTO auth.accounts(email) VALUES('legacy@example.com') RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO auth.password_credentials(account_id,password_hash) VALUES($1,'legacy-attacker-hash')").bind(account).execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO auth.one_time_tokens(account_id,purpose,token_hash,sent_to_email,expires_at) VALUES($1,'email_verification',$2,'legacy@example.com',clock_timestamp()+interval '1 hour')").bind(account).bind(vec![1u8;32]).execute(&pool).await.unwrap();
    let id = start(&state, "legacy@example.com", "OwnerPass456!").await;
    let code = mail.last();
    assert_eq!(
        state
            .email_verification_service
            .confirm(id, &code)
            .await
            .unwrap()
            .id,
        account
    );
    assert!(state
        .auth_service
        .register(
            "legacy@example.com".into(),
            "AttackerPass123!".into(),
            &TestBudget
        )
        .await
        .is_err());
    assert!(state
        .email_verification_service
        .resend(id, &TestBudget)
        .await
        .is_err());
    let hash: String = sqlx::query_scalar(
        "SELECT password_hash FROM auth.password_credentials WHERE account_id=$1",
    )
    .bind(account)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(Argon2PasswordHasher
        .verify_password("OwnerPass456!", &hash)
        .unwrap());
    assert!(sqlx::query_scalar::<_, bool>(
        "SELECT invalidated_at IS NOT NULL FROM auth.one_time_tokens WHERE account_id=$1"
    )
    .bind(account)
    .fetch_one(&pool)
    .await
    .unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn mail_failure_keeps_attempt_and_can_retry_without_open_transaction(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    mail.fail.store(true, Ordering::SeqCst);
    let state = state(&pool, mail.clone());
    let app = crate::routes::router(state.clone());
    let response = call(
        &app,
        "/auth/register",
        serde_json::json!({"email":"mail@example.com","password":"ValidPass123!"}),
        StatusCode::CREATED,
    )
    .await;
    assert_eq!(response["verification_email_sent"], false);
    let id = response["registration_attempt_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    age(&pool, id).await;
    mail.fail.store(false, Ordering::SeqCst);
    call(
        &app,
        "/auth/resend-verification",
        serde_json::json!({"registration_attempt_id":id}),
        StatusCode::ACCEPTED,
    )
    .await;
    state
        .email_verification_service
        .confirm(id, &mail.last())
        .await
        .unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn database_error_rolls_back_activation_code_consumption_and_password(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    let state = state(&pool, mail.clone());
    let id = start(&state, "rollback@example.com", "ValidPass123!").await;
    let code = mail.last();
    sqlx::query("CREATE FUNCTION auth.fail_password() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'test-only failure'; END; $$").execute(&pool).await.unwrap();
    sqlx::query("CREATE TRIGGER fail_password BEFORE INSERT ON auth.password_credentials FOR EACH ROW EXECUTE FUNCTION auth.fail_password()").execute(&pool).await.unwrap();
    assert!(matches!(
        state.email_verification_service.confirm(id, &code).await,
        Err(VerificationError::Database(_))
    ));
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth.accounts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(sqlx::query_scalar::<_, bool>(
        "SELECT consumed_at IS NULL FROM auth.registration_attempts WHERE id=$1"
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap());
    sqlx::query("DROP TRIGGER fail_password ON auth.password_credentials")
        .execute(&pool)
        .await
        .unwrap();
    state
        .email_verification_service
        .confirm(id, &code)
        .await
        .unwrap();
}

struct Budget(Result<bool, ()>);

struct LockCheckingMail {
    pool: PgPool,
    codes: Mutex<Vec<String>>,
}
#[async_trait::async_trait]
impl VerificationEmailSender for LockCheckingMail {
    async fn send(&self, email: &str, code: &str, _: &str) -> Result<(), EmailDeliveryError> {
        let mut tx = self.pool.begin().await.unwrap();
        let acquired: bool =
            sqlx::query_scalar("SELECT pg_try_advisory_xact_lock(hashtextextended($1,17005))")
                .bind(email)
                .fetch_one(&mut *tx)
                .await
                .unwrap();
        assert!(
            acquired,
            "Mail delivery must run after the registration transaction releases its lock"
        );
        tx.rollback().await.unwrap();
        self.codes.lock().unwrap().push(code.into());
        Ok(())
    }
}
#[sqlx::test(migrations = "./migrations")]
async fn initial_and_resend_mail_run_after_commit_without_registration_locks(pool: PgPool) {
    let mail = Arc::new(LockCheckingMail {
        pool: pool.clone(),
        codes: Mutex::new(Vec::new()),
    });
    let verification = EmailVerificationService::new(
        RegistrationAttemptsRepository::new(pool.clone()),
        mail.clone(),
        &[0x17; 32],
    )
    .unwrap();
    let auth = AuthService::new(
        AccountsRepository::new(pool.clone()),
        Arc::new(Argon2PasswordHasher),
        verification.clone(),
        4,
    );
    let (attempt, sent) = auth
        .register(
            "locks@example.com".into(),
            "ValidPass123!".into(),
            &TestBudget,
        )
        .await
        .unwrap();
    assert!(sent);
    age(&pool, attempt.id).await;
    assert!(
        verification
            .resend(attempt.id, &TestBudget)
            .await
            .unwrap()
            .1
    );
    let code = mail.codes.lock().unwrap().last().unwrap().clone();
    verification.confirm(attempt.id, &code).await.unwrap();
}
#[async_trait::async_trait]
impl RecipientBudget for Budget {
    async fn consume(&self, _: &str) -> Result<bool, ()> {
        self.0
    }
}
#[sqlx::test(migrations = "./migrations")]
async fn recipient_failure_or_exhaustion_never_creates_attempt_or_sends_mail(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    let state = state(&pool, mail.clone());
    for result in [Ok(false), Err(())] {
        assert!(state
            .auth_service
            .register(
                "budget@example.com".into(),
                "ValidPass123!".into(),
                &Budget(result)
            )
            .await
            .is_err());
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth.registration_attempts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(mail.messages.lock().unwrap().is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn legacy_paths_and_malformed_inputs_never_activate_or_expose_secrets(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    let state = state(&pool, mail.clone());
    let id = start(&state, "legacy-path@example.com", "ValidPass123!").await;
    let code = mail.last();
    let app = crate::routes::router(state.clone());
    for uri in [
        "/auth/verify-email",
        "/auth/verify-email?token=legacy-secret",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(uri)
                    .extension(axum::extract::ConnectInfo(
                        "192.0.2.17:4017".parse::<std::net::SocketAddr>().unwrap(),
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::GONE);
        let bytes = to_bytes(response.into_body(), 16384).await.unwrap();
        assert!(!String::from_utf8_lossy(&bytes).contains("legacy-secret"));
    }
    for body in [
        serde_json::json!({"token":"legacy-secret"}),
        serde_json::json!({"registration_attempt_id":"malformed","code":code}),
        serde_json::json!({"registration_attempt_id":id,"code":code,"password":"leaked"}),
    ] {
        let problem = call(&app, "/auth/verify-email", body, StatusCode::BAD_REQUEST).await;
        assert!(!problem.to_string().contains(&code));
        assert!(!problem.to_string().contains("leaked"));
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth.accounts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    state
        .email_verification_service
        .confirm(id, &code)
        .await
        .unwrap();
}

#[tokio::test]
async fn jwks_route_is_public_even_when_redis_and_database_are_unavailable() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://test:test@127.0.0.1:1/unavailable")
        .unwrap();
    let app = crate::routes::router(state(&pool, Arc::new(Mail::default())));
    let response = app
        .oneshot(
            Request::builder()
                .uri("/.well-known/jwks.json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["cache-control"],
        "public, max-age=300, must-revalidate"
    );
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Redis"]
async fn initial_delivery_and_resends_share_recipient_budget(pool: PgPool) {
    let mail = Arc::new(Mail::default());
    let mut state = state(&pool, mail.clone());
    state.auth_rate_limiter = Arc::new(
        crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(
            std::env::var("REDIS_URL").unwrap(),
        )
        .with_recipient_policy_for_tests(
            crate::protection::rate_limit::BucketPolicy::new(1, 600).unwrap(),
        ),
    );
    let app = crate::routes::router(state.clone());
    let email = format!("budget-{}@example.com", Uuid::new_v4());
    let registered = call(
        &app,
        "/auth/register",
        serde_json::json!({"email":email,"password":"ValidPass123!"}),
        StatusCode::CREATED,
    )
    .await;
    let id: Uuid = registered["registration_attempt_id"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    let code = mail.last();
    age(&pool, id).await;
    call(
        &app,
        "/auth/resend-verification",
        serde_json::json!({"registration_attempt_id":id}),
        StatusCode::TOO_MANY_REQUESTS,
    )
    .await;
    call(
        &app,
        "/auth/register",
        serde_json::json!({"email":email,"password":"OtherPass456!"}),
        StatusCode::TOO_MANY_REQUESTS,
    )
    .await;
    assert_eq!(mail.messages.lock().unwrap().len(), 1);
    state
        .email_verification_service
        .confirm(id, &code)
        .await
        .unwrap();
}

#[derive(Clone)]
struct Logs(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Logs {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
#[sqlx::test(migrations = "./migrations")]
async fn registration_errors_do_not_log_codes_passwords_or_mac(pool: PgPool) {
    use tracing::instrument::WithSubscriber;
    let mail = Arc::new(Mail::default());
    let state = state(&pool, mail.clone());
    let id = start(&state, "logs@example.com", "SecretPass123!").await;
    let code = mail.last();
    let mac: Vec<u8> =
        sqlx::query_scalar("SELECT code_mac FROM auth.registration_attempts WHERE id=$1")
            .bind(id)
            .fetch_one(&pool)
            .await
            .unwrap();
    let mac_hex: String = mac.iter().map(|b| format!("{b:02x}")).collect();
    let logs = Logs(Arc::new(Mutex::new(Vec::new())));
    let writer = logs.clone();
    let subscriber = tracing_subscriber::fmt()
        .without_time()
        .with_ansi(false)
        .with_max_level(tracing::Level::TRACE)
        .with_writer(move || writer.clone())
        .finish();
    let app = crate::routes::router(state);
    async {
        call(
            &app,
            "/auth/verify-email",
            serde_json::json!({"registration_attempt_id":id,"code":"SensitiveWrongCode"}),
            StatusCode::BAD_REQUEST,
        )
        .await;
        pool.close().await;
        call(
            &app,
            "/auth/verify-email",
            serde_json::json!({"registration_attempt_id":id,"code":code}),
            StatusCode::SERVICE_UNAVAILABLE,
        )
        .await;
    }
    .with_subscriber(subscriber)
    .await;
    let bytes = logs.0.lock().unwrap();
    let output = String::from_utf8_lossy(&bytes);
    for secret in [&code, "SensitiveWrongCode", "SecretPass123!", &mac_hex] {
        assert!(!output.contains(secret));
    }
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires isolated Mailpit SMTP and HTTP API"]
async fn real_mailpit_code_completes_registration_without_activation_links(pool: PgPool) {
    let port = std::env::var("AUTH_TEST_MAILPIT_SMTP_PORT")
        .unwrap()
        .parse()
        .unwrap();
    let mail = crate::providers::mailpit::MailpitEmailSender::new(
        "127.0.0.1".into(),
        port,
        "Sonimbus <noreply@example.com>".into(),
    )
    .unwrap();
    let service = EmailVerificationService::new(
        RegistrationAttemptsRepository::new(pool.clone()),
        Arc::new(mail),
        &[0x17; 32],
    )
    .unwrap();
    let auth = AuthService::new(
        AccountsRepository::new(pool.clone()),
        Arc::new(Argon2PasswordHasher),
        service.clone(),
        4,
    );
    let email = format!("mailpit-{}@example.com", Uuid::new_v4());
    let (attempt, sent) = auth
        .register(email.clone(), "ValidPass123!".into(), &TestBudget)
        .await
        .unwrap();
    assert!(sent);
    let base = std::env::var("AUTH_TEST_MAILPIT_API_URL").unwrap();
    let client = reqwest::Client::new();
    let search: serde_json::Value = client
        .get(format!("{base}/api/v1/search"))
        .query(&[("query", format!("to:{email}"))])
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let message_id = search["messages"][0]["ID"].as_str().unwrap();
    let message: serde_json::Value = client
        .get(format!("{base}/api/v1/message/{message_id}"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let text = message["Text"].as_str().unwrap();
    let code = text
        .split_whitespace()
        .find(|s| s.len() == 8 && s.bytes().all(|b| b.is_ascii_digit()))
        .unwrap();
    assert!(!message["HTML"].as_str().unwrap().contains("href="));
    service.confirm(attempt.id, code).await.unwrap();
}
