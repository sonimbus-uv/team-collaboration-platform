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
        accounts_repository::{AccountsRepository, CreateAccountError},
        one_time_tokens_repository::OneTimeTokensRepository,
    },
    security::password_hasher::{PasswordHasher, PasswordHashingError},
    services::{
        account_service::AuthService,
        email_verification_service::{new_token, EmailVerificationService},
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

    AppState {
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

fn post(path: &str, body: serde_json::Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn email_link_confirms_and_head_does_not_consume(pool: PgPool) {
    let sender = Arc::new(FakeSender::default());
    let state = state(pool.clone(), sender.clone());

    state
        .auth_service
        .register("link@example.com".into(), "ValidPass123!".into())
        .await
        .unwrap();

    let token = sender.tokens.lock().unwrap()[0].clone();
    let app = crate::routes::router(state);
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