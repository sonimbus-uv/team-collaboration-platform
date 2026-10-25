//! Each sqlx test creates an isolated database; DATABASE_URL needs CREATEDB.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::{
    ports::email_verification::{EmailDeliveryError, RecipientBudget, VerificationEmailSender},
    repositories::{
        accounts_repository::AccountsRepository,
        one_time_tokens_repository::OneTimeTokensRepository,
    },
    security::password_hasher::{PasswordHasher, PasswordHashingError},
    services::{
        account_service::AuthService, email_verification_service::EmailVerificationService,
    },
    state::AppState,
};

#[derive(Default)]
struct FakeSender {
    tokens: Mutex<Vec<String>>,
    fail: bool,
}

#[async_trait::async_trait]
impl VerificationEmailSender for FakeSender {
    async fn send(&self, _: &str, token: &str, _: &str) -> Result<(), EmailDeliveryError> {
        if self.fail {
            return Err(EmailDeliveryError::Transport);
        }

        self.tokens.lock().unwrap().push(token.into());

        Ok(())
    }
}

struct FakeHasher;

struct FakeBudget {
    calls: AtomicUsize,
    outcome: Result<bool, ()>,
}

impl FakeBudget {
    fn new(outcome: Result<bool, ()>) -> Self {
        Self {
            calls: AtomicUsize::new(0),
            outcome,
        }
    }
}

#[async_trait::async_trait]
impl RecipientBudget for FakeBudget {
    async fn consume(&self, _: &str) -> Result<bool, ()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        self.outcome
    }
}

async fn registered_service(
    pool: &PgPool,
    email: &str,
) -> (EmailVerificationService, Arc<FakeSender>) {
    let sender = Arc::new(FakeSender::default());
    let verification =
        EmailVerificationService::new(OneTimeTokensRepository::new(pool.clone()), sender.clone());
    AuthService::new(
        AccountsRepository::new(pool.clone()),
        Arc::new(FakeHasher),
        verification.clone(),
        4,
    )
    .register(email.into(), "ValidPass123!".into())
    .await
    .unwrap();
    (verification, sender)
}

async fn age_tokens(pool: &PgPool, email: &str) {
    sqlx::query(
        "UPDATE auth.one_time_tokens SET created_at = now() - interval '61 seconds' WHERE sent_to_email = $1",
    ).bind(email).execute(pool).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn ineligible_resends_do_not_consume_budget(pool: PgPool) {
    let email = "cooldown@example.com";
    let (service, sender) = registered_service(&pool, email).await;
    let budget = FakeBudget::new(Ok(true));
    service
        .resend("missing@example.com", &budget)
        .await
        .unwrap();
    service.resend(email, &budget).await.unwrap();
    assert_eq!(budget.calls.load(Ordering::SeqCst), 0);
    assert_eq!(sender.tokens.lock().unwrap().len(), 1);

    let token = sender.tokens.lock().unwrap()[0].clone();
    service.confirm(&token).await.unwrap();
    age_tokens(&pool, email).await;
    service.resend(email, &budget).await.unwrap();
    assert_eq!(budget.calls.load(Ordering::SeqCst), 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn unavailable_or_exhausted_budget_preserves_token(pool: PgPool) {
    for (index, outcome) in [Ok(false), Err(())].into_iter().enumerate() {
        let email = format!("budget-{index}@example.com");
        let (service, sender) = registered_service(&pool, &email).await;
        let token = sender.tokens.lock().unwrap()[0].clone();
        age_tokens(&pool, &email).await;
        let budget = FakeBudget::new(outcome);
        service.resend(&email, &budget).await.unwrap();
        assert_eq!(budget.calls.load(Ordering::SeqCst), 1);
        assert_eq!(sender.tokens.lock().unwrap().len(), 1);
        service.confirm(&token).await.unwrap();
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_resends_send_once_and_preserve_both_links(pool: PgPool) {
    // Exercise confirmation with either the original or replacement link.
    for chosen in 0..2 {
        let email = format!("concurrent-{chosen}@example.com");
        let (service, sender) = registered_service(&pool, &email).await;
        age_tokens(&pool, &email).await;
        let budget = FakeBudget::new(Ok(true));
        let (first, second) = tokio::join!(
            service.resend(&email, &budget),
            service.resend(&email, &budget),
        );
        first.unwrap();
        second.unwrap();
        assert_eq!(budget.calls.load(Ordering::SeqCst), 1);
        let tokens = sender.tokens.lock().unwrap().clone();
        assert_eq!(tokens.len(), 2);
        let pending: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM auth.one_time_tokens WHERE sent_to_email = $1 AND used_at IS NULL AND invalidated_at IS NULL",
        ).bind(&email).fetch_one(&pool).await.unwrap();
        assert_eq!(pending, 2);
        service.confirm(&tokens[chosen]).await.unwrap();
        assert!(matches!(
            service.confirm(&tokens[1 - chosen]).await,
            Err(crate::services::email_verification_service::VerificationError::InvalidToken)
        ));
        let invalidated: i64 = sqlx::query_scalar(
            "SELECT count(*) FROM auth.one_time_tokens WHERE sent_to_email = $1 AND invalidated_at IS NOT NULL",
        ).bind(&email).fetch_one(&pool).await.unwrap();
        assert_eq!(invalidated, 1);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn delivery_failure_preserves_original_link(pool: PgPool) {
    let email = "delivery-failure@example.com";
    let (_, sender) = registered_service(&pool, email).await;
    let token = sender.tokens.lock().unwrap()[0].clone();
    age_tokens(&pool, email).await;
    let failing_service = EmailVerificationService::new(
        OneTimeTokensRepository::new(pool),
        Arc::new(FakeSender {
            tokens: Mutex::new(Vec::new()),
            fail: true,
        }),
    );
    failing_service
        .resend(email, &FakeBudget::new(Ok(true)))
        .await
        .unwrap();
    failing_service.confirm(&token).await.unwrap();
}

#[sqlx::test(migrations = "./migrations")]
async fn redis_outage_keeps_real_verification_available(pool: PgPool) {
    let email = "redis-outage@example.com";
    let (_, sender) = registered_service(&pool, email).await;
    let token = sender.tokens.lock().unwrap()[0].clone();
    // Build the complete application without requiring a live Redis connection.
    let verification =
        EmailVerificationService::new(OneTimeTokensRepository::new(pool.clone()), sender);
    let app = crate::routes::router(AppState {
        login_enabled: false,
        token_issuer: crate::tests::token_support::issuer(),
        pool: pool.clone(),
        auth_service: AuthService::new(
            AccountsRepository::new(pool),
            Arc::new(FakeHasher),
            verification.clone(),
            4,
        ),
        email_verification_service: verification,
        auth_rate_limiter: Arc::new(
            crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(
                "redis://127.0.0.1:1/0".into(),
            ),
        ),
    });
    for path in [
        "/auth/register",
        "/auth/resend-verification",
        "/health/redis",
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(if path.starts_with("/health") {
                        "GET"
                    } else {
                        "POST"
                    })
                    .uri(path)
                    .extension(axum::extract::ConnectInfo(
                        "192.0.2.55:45000".parse::<std::net::SocketAddr>().unwrap(),
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
    }
    let response = app
        .oneshot(
            Request::builder()
                .uri(format!("/auth/verify-email?token={token}"))
                .extension(axum::extract::ConnectInfo(
                    "192.0.2.55:45000".parse::<std::net::SocketAddr>().unwrap(),
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
}

impl PasswordHasher for FakeHasher {
    fn hash_password(&self, _: &str) -> Result<String, PasswordHashingError> {
        Ok("test-only-hash".into())
    }

    fn verify_password(&self, _: &str, _: &str) -> Result<bool, PasswordHashingError> {
        Ok(false)
    }
}

#[tokio::test]
async fn jwks_route_is_public_even_when_redis_and_database_are_unavailable() {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://test:test@127.0.0.1:1/unavailable")
        .unwrap();
    let verification = EmailVerificationService::new(
        OneTimeTokensRepository::new(pool.clone()),
        Arc::new(FakeSender::default()),
    );
    let app = crate::routes::router(AppState {
        login_enabled: false,
        token_issuer: crate::tests::token_support::issuer(),
        auth_service: AuthService::new(
            AccountsRepository::new(pool.clone()),
            Arc::new(FakeHasher),
            verification.clone(),
            4,
        ),
        email_verification_service: verification,
        pool,
        auth_rate_limiter: Arc::new(
            crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(
                "redis://127.0.0.1:1/0".into(),
            ),
        ),
    });
    for _ in 0..25 {
        // No Authorization or ConnectInfo: the full router must bypass Auth budgets.
        let response = app
            .clone()
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
}

fn state(pool: PgPool, sender: Arc<FakeSender>) -> AppState {
    let verification =
        EmailVerificationService::new(OneTimeTokensRepository::new(pool.clone()), sender);

    let redis_url = std::env::var("REDIS_URL").expect("Router integration tests require REDIS_URL");

    AppState {
        login_enabled: false,
        token_issuer: crate::tests::token_support::issuer(),
        auth_rate_limiter: Arc::new(
            crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(redis_url),
        ),
        auth_service: AuthService::new(
            AccountsRepository::new(pool.clone()),
            Arc::new(FakeHasher),
            verification.clone(),
            4,
        ),
        email_verification_service: verification,
        pool,
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn email_link_confirms_and_head_does_not_consume(pool: PgPool) {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("Invalid local environment file: {error}"),
    }

    let sender = Arc::new(FakeSender::default());
    let state = state(pool.clone(), sender.clone());

    state
        .auth_service
        .register("link@example.com".into(), "ValidPass123!".into())
        .await
        .unwrap();

    let token = sender.tokens.lock().unwrap()[0].clone();
    let test_peer: std::net::SocketAddr = format!(
        "[2001:db8::{:x}:{:x}:{:x}:{:x}]:45000",
        rand::random::<u16>(),
        rand::random::<u16>(),
        rand::random::<u16>(),
        rand::random::<u16>(),
    )
    .parse()
    .unwrap();

    let app =
        crate::routes::router(state).layer(axum::Extension(axum::extract::ConnectInfo(test_peer)));

    let uri = format!("/auth/verify-email?token={token}");

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("HEAD")
                .uri(&uri)
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);

    let used: bool = sqlx::query_scalar("SELECT used_at IS NOT NULL FROM auth.one_time_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();

    assert!(!used);

    for path in [
        "/auth/verify-email",
        "/auth/verify-email?token=",
        "/auth/verify-email?token=bad",
        "/auth/verify-email?token=bad&token=other",
    ] {
        let response = app
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            response.headers()["content-type"],
            "text/html; charset=utf-8"
        );
    }

    let response = app
        .clone()
        .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    assert!(response.headers().contains_key("content-security-policy"));

    let body = to_bytes(response.into_body(), 8192).await.unwrap();
    let html = String::from_utf8_lossy(&body);

    assert!(html.contains("Correo verificado"));
    assert!(!html.contains(&token));
    assert!(!html.contains("link@example.com"));

    let verified: bool =
        sqlx::query_scalar("SELECT email_verified_at IS NOT NULL FROM auth.accounts")
            .fetch_one(&pool)
            .await
            .unwrap();

    assert!(verified);

    let response = app
        .oneshot(Request::builder().uri(&uri).body(Body::empty()).unwrap())
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[sqlx::test(migrations = "./migrations")]
async fn exhausted_recipient_budget_preserves_existing_verification_link(pool: PgPool) {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(dotenvy::Error::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("Invalid local environment file: {error}"),
    }

    let sender = Arc::new(FakeSender::default());
    let mut app_state = state(pool.clone(), sender.clone());

    let redis_url = std::env::var("REDIS_URL").expect("Router integration tests require REDIS_URL");

    let recipient_policy = crate::protection::rate_limit::BucketPolicy::new(1, 3600).unwrap();

    app_state.auth_rate_limiter = Arc::new(
        crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(redis_url)
            .with_recipient_policy_for_tests(recipient_policy),
    );

    let limiter = app_state.auth_rate_limiter.clone();
    let app = crate::routes::router(app_state);

    let email = format!("recipient-{:032x}@example.com", rand::random::<u128>());

    let request = |method: &str, uri: &str, body: Body, peer: &str| {
        Request::builder()
            .method(method)
            .uri(uri)
            .header("content-type", "application/json")
            .extension(axum::extract::ConnectInfo(
                peer.parse::<std::net::SocketAddr>().unwrap(),
            ))
            .body(body)
            .unwrap()
    };

    let register_body = serde_json::json!({
        "email": &email,
        "password": "ValidPass123!"
    });

    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/auth/register",
            Body::from(register_body.to_string()),
            "192.0.2.60:45000",
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(sender.tokens.lock().unwrap().len(), 1);

    let token = sender.tokens.lock().unwrap()[0].clone();

    assert_eq!(
        limiter.check_recipient(&email).await.unwrap(),
        crate::protection::rate_limit::RateLimitDecision::Allowed
    );

    let aged = sqlx::query(
        r#"
        UPDATE auth.one_time_tokens
        SET created_at = now() - interval '61 seconds'
        WHERE sent_to_email = $1
          AND purpose = 'email_verification'
        "#,
    )
    .bind(&email)
    .execute(&pool)
    .await
    .unwrap();

    assert_eq!(aged.rows_affected(), 1);

    let resend_body = serde_json::json!({
        "email": format!(" {} ", email.to_uppercase())
    });

    let response = app
        .clone()
        .oneshot(request(
            "POST",
            "/auth/resend-verification",
            Body::from(resend_body.to_string()),
            "192.0.2.61:45000",
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::ACCEPTED);
    assert!(!response.headers().contains_key("retry-after"));

    assert_eq!(
        sender.tokens.lock().unwrap().len(),
        1,
        "A rejected resend must not send another email"
    );

    let token_state: (i64, bool) = sqlx::query_as(
        r#"
        SELECT
            count(*),
            COALESCE(
                bool_and(
                    used_at IS NULL
                    AND invalidated_at IS NULL
                ),
                false
            )
        FROM auth.one_time_tokens
        WHERE sent_to_email = $1
          AND purpose = 'email_verification'
        "#,
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert_eq!(token_state.0, 1, "No replacement token must be created");
    assert!(
        token_state.1,
        "The original token must remain unused and valid"
    );

    let verification_uri = format!("/auth/verify-email?token={token}");

    let response = app
        .oneshot(request(
            "GET",
            &verification_uri,
            Body::empty(),
            "192.0.2.62:45000",
        ))
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        response.headers()["content-type"],
        "text/html; charset=utf-8"
    );

    let verified: bool = sqlx::query_scalar(
        r#"
        SELECT email_verified_at IS NOT NULL
        FROM auth.accounts
        WHERE email = $1
        "#,
    )
    .bind(&email)
    .fetch_one(&pool)
    .await
    .unwrap();

    assert!(verified);

    assert_eq!(sender.tokens.lock().unwrap().len(), 1);
}
