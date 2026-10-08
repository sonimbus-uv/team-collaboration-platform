//! Each sqlx test creates an isolated database; DATABASE_URL needs CREATEDB.
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
use axum::{
    body::{to_bytes, Body},
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

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
async fn register_confirm_and_reject_reuse(pool: PgPool) {
    let sender = Arc::new(FakeSender::default());
    let app = crate::routes::router(state(pool.clone(), sender.clone()));
    let response = app
        .clone()
        .oneshot(post(
            "/auth/register",
            serde_json::json!({"email":" TEST@Example.COM ","password":"ValidPass123!"}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(body["email"], "test@example.com");
    assert_eq!(body["email_verified"], false);
    assert_eq!(body["verification_email_sent"], true);
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/auth/register",
                serde_json::json!({"email":"test@example.com","password":"ValidPass123!"})
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::CONFLICT
    );
    let token = sender.tokens.lock().unwrap()[0].clone();
    let stored: Vec<u8> = sqlx::query_scalar("SELECT token_hash FROM auth.one_time_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_ne!(stored, token.as_bytes());
    let response = app
        .clone()
        .oneshot(post(
            "/auth/verify-email",
            serde_json::json!({"token":token}),
        ))
        .await
        .unwrap();
    assert_eq!(response.status(), StatusCode::OK);
    let body: serde_json::Value =
        serde_json::from_slice(&to_bytes(response.into_body(), 8192).await.unwrap()).unwrap();
    assert_eq!(body["email_verified"], true);
    assert_eq!(
        app.oneshot(post(
            "/auth/verify-email",
            serde_json::json!({"token":token})
        ))
        .await
        .unwrap()
        .status(),
        StatusCode::BAD_REQUEST
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn duplicates_are_safe_under_concurrency(pool: PgPool) {
    let repository = AccountsRepository::new(pool.clone());
    let (_, first) = new_token();
    let (_, second) = new_token();
    let (a, b) = tokio::join!(
        repository.create_with_password("same@example.com", "hash", &first),
        repository.create_with_password("SAME@example.com", "hash", &second)
    );
    assert!(matches!(
        (&a, &b),
        (Ok(_), Err(CreateAccountError::EmailAlreadyExists))
            | (Err(CreateAccountError::EmailAlreadyExists), Ok(_))
    ));
    for table in ["accounts", "password_credentials", "one_time_tokens"] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT count(*) FROM auth.{table}"))
            .fetch_one(&pool)
            .await
            .unwrap();
        assert_eq!(count, 1);
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn token_failure_rolls_back_account_and_password(pool: PgPool) {
    let repository = AccountsRepository::new(pool.clone());
    let (_, hash) = new_token();
    repository
        .create_with_password("first@example.com", "hash", &hash)
        .await
        .unwrap();
    assert!(repository
        .create_with_password("second@example.com", "hash", &hash)
        .await
        .is_err());
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM auth.accounts WHERE email = 'second@example.com'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 0);
    let credentials: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.password_credentials")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(credentials, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn resend_invalidates_old_token_and_enforces_cooldown(pool: PgPool) {
    let sender = Arc::new(FakeSender::default());
    let state = state(pool.clone(), sender.clone());
    state
        .auth_service
        .register("test@example.com".into(), "ValidPass123!".into())
        .await
        .unwrap();
    let old = sender.tokens.lock().unwrap()[0].clone();
    state
        .email_verification_service
        .resend("test@example.com")
        .await
        .unwrap();
    assert_eq!(sender.tokens.lock().unwrap().len(), 1);
    sqlx::query("UPDATE auth.one_time_tokens SET created_at = now() - interval '2 minutes'")
        .execute(&pool)
        .await
        .unwrap();
    let (a, b) = tokio::join!(
        state.email_verification_service.resend("test@example.com"),
        state.email_verification_service.resend("test@example.com")
    );
    a.unwrap();
    b.unwrap();
    assert_eq!(sender.tokens.lock().unwrap().len(), 2);
    assert!(state
        .email_verification_service
        .confirm(&old)
        .await
        .is_err());
    let current = sender.tokens.lock().unwrap()[1].clone();
    let (a, b) = tokio::join!(
        state.email_verification_service.confirm(&current),
        state.email_verification_service.confirm(&current)
    );
    assert_ne!(a.is_ok(), b.is_ok());
}

#[sqlx::test(migrations = "./migrations")]
async fn expiry_email_change_and_delivery_failure(pool: PgPool) {
    let sender = Arc::new(FakeSender::default());
    let state = state(pool.clone(), sender.clone());
    state
        .auth_service
        .register("test@example.com".into(), "ValidPass123!".into())
        .await
        .unwrap();
    let token = sender.tokens.lock().unwrap()[0].clone();
    sqlx::query("UPDATE auth.accounts SET email = 'changed@example.com'")
        .execute(&pool)
        .await
        .unwrap();
    assert!(state
        .email_verification_service
        .confirm(&token)
        .await
        .is_err());
    sqlx::query("UPDATE auth.accounts SET email = 'test@example.com'")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE auth.one_time_tokens SET created_at = now() - interval '2 days', expires_at = now() - interval '1 day'").execute(&pool).await.unwrap();
    assert!(state
        .email_verification_service
        .confirm(&token)
        .await
        .is_err());
    let failed_state = self::state(
        pool.clone(),
        Arc::new(FakeSender {
            fail: true,
            ..Default::default()
        }),
    );
    let (_, sent) = failed_state
        .auth_service
        .register("failed@example.com".into(), "ValidPass123!".into())
        .await
        .unwrap();
    assert!(!sent);
    let count: i64 =
        sqlx::query_scalar("SELECT count(*) FROM auth.accounts WHERE email = 'failed@example.com'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(count, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn handler_rejects_invalid_input_and_hides_database_errors(pool: PgPool) {
    let app = crate::routes::router(state(pool.clone(), Arc::new(FakeSender::default())));
    assert_eq!(
        app.clone()
            .oneshot(post(
                "/auth/register",
                serde_json::json!({"email":"a@b..com","password":"ValidPass123!"})
            ))
            .await
            .unwrap()
            .status(),
        StatusCode::BAD_REQUEST
    );
    sqlx::query("DROP TABLE auth.password_credentials")
        .execute(&pool)
        .await
        .unwrap();
    let response = app
        .oneshot(post(
            "/auth/register",
            serde_json::json!({"email":"a@b.com","password":"ValidPass123!"}),
        ))
        .await
        .unwrap();
    assert!(response.status().is_server_error());
    let body = to_bytes(response.into_body(), 8192).await.unwrap();
    assert!(!String::from_utf8_lossy(&body).contains("password_credentials"));
}
