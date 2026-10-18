//! Each sqlx test creates an isolated database; DATABASE_URL needs CREATEDB.

use std::sync::{Arc, Mutex};

use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::{
    ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
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

impl PasswordHasher for FakeHasher {
    fn hash_password(&self, _: &str) -> Result<String, PasswordHashingError> {
        Ok("test-only-hash".into())
    }

    fn verify_password(&self, _: &str, _: &str) -> Result<bool, PasswordHashingError> {
        Ok(false)
    }
}

fn state(pool: PgPool, sender: Arc<FakeSender>) -> AppState {
    let verification =
        EmailVerificationService::new(OneTimeTokensRepository::new(pool.clone()), sender);

    let redis_url = std::env::var("REDIS_URL")
        .expect("Router integration tests require REDIS_URL");

    AppState {
        auth_rate_limiter: Arc::new(
            crate::auth_rate_limiter::AuthRateLimiter::for_tests(redis_url),
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
        Err(dotenvy::Error::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound => {}
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

    let app = crate::routes::router(state).layer(axum::Extension(
        axum::extract::ConnectInfo(test_peer),
    ));

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
async fn exhausted_recipient_budget_preserves_existing_verification_link(
    pool: PgPool,
) {
    match dotenvy::dotenv() {
        Ok(_) => {}
        Err(dotenvy::Error::Io(error))
            if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => panic!("Invalid local environment file: {error}"),
    }

    let sender = Arc::new(FakeSender::default());
    let mut app_state = state(pool.clone(), sender.clone());

    let redis_url = std::env::var("REDIS_URL")
        .expect("Router integration tests require REDIS_URL");

    let recipient_policy =
        crate::rate_limit::BucketPolicy::new(1, 3600).unwrap();

    app_state.auth_rate_limiter = Arc::new(
        crate::auth_rate_limiter::AuthRateLimiter::for_tests(redis_url)
            .with_recipient_policy_for_tests(recipient_policy),
    );

    let app = crate::routes::router(app_state);

    let email = format!(
        "recipient-{:032x}@example.com",
        rand::random::<u128>()
    );

    let request = |
        method: &str,
        uri: &str,
        body: Body,
        peer: &str,
    | {
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

    assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(
        response.headers()["content-type"],
        "application/problem+json"
    );

    let retry_after: u64 = response.headers()["retry-after"]
        .to_str()
        .unwrap()
        .parse()
        .unwrap();

    assert!(retry_after > 0);
    assert!(retry_after <= 3600);

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
