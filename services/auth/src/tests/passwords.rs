//! Real PostgreSQL transactions; no production credentials or user data are used.
use crate::{
    dto::passwords::ResetPassword,
    ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
    protection::auth_rate_limiter::AuthRateLimiter,
    repositories::{
        accounts_repository::AccountsRepository,
        registration_attempts_repository::RegistrationAttemptsRepository,
    },
    security::password_hasher::{Argon2PasswordHasher, PasswordHasher},
    services::{
        account_service::AuthService,
        email_verification_service::EmailVerificationService,
        password_service::{PasswordError, PasswordService},
    },
    state::AppState,
};
use axum::{
    body::{to_bytes, Body},
    extract::ConnectInfo,
    http::{Request, StatusCode},
};
use sqlx::PgPool;
use std::sync::{Arc, Mutex};
use tokio::sync::Notify;
use tower::ServiceExt;
use uuid::Uuid;

#[derive(Default)]
struct Mail {
    messages: Mutex<Vec<(String, String)>>,
    delivered: Notify,
}

#[async_trait::async_trait]
impl VerificationEmailSender for Mail {
    async fn send(&self, _: &str, _: &str, _: &str) -> Result<(), EmailDeliveryError> {
        Ok(())
    }
    async fn send_recovery(
        &self,
        email: &str,
        code: &str,
        _: &str,
    ) -> Result<(), EmailDeliveryError> {
        self.messages
            .lock()
            .unwrap()
            .push((email.into(), code.into()));
        self.delivered.notify_one();
        Ok(())
    }
}

impl Mail {
    async fn code(&self) -> String {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            loop {
                let delivered = self.delivered.notified();
                let code = self
                    .messages
                    .lock()
                    .unwrap()
                    .last()
                    .map(|(_, code)| code.clone());
                if let Some(code) = code {
                    return code;
                }
                delivered.await;
            }
        })
        .await
        .expect("Recovery email was not delivered before the test deadline")
    }
}

fn fixture(pool: &PgPool) -> (AppState, Arc<Mail>) {
    let mail = Arc::new(Mail::default());
    let limiter = Arc::new(
        AuthRateLimiter::for_tests("redis://127.0.0.1:1/0".into()).without_redis_for_tests(),
    );
    let verification = EmailVerificationService::new(
        RegistrationAttemptsRepository::new(pool.clone()),
        mail.clone(),
        &[0x17; 32],
    )
    .unwrap();
    let auth = AuthService::new(
        AccountsRepository::new(pool.clone()),
        crate::services::password_processor::PasswordProcessor::new(
            Arc::new(Argon2PasswordHasher),
            4,
        ),
        verification.clone(),
    );
    let service = PasswordService::new(
        crate::services::password_service::PasswordServiceDependencies {
            pool: pool.clone(),
            passwords: auth.password_processor(),
            sender: mail.clone(),
            limiter: limiter.clone(),
        },
        &[0x22; 32],
    )
    .unwrap();
    (
        AppState {
            google_service: None,
            contact_email_service: None,
            security_notifications: None,
            password_service: Some(service),
            login_enabled: true,
            token_issuer: super::token_support::issuer(),
            pool: pool.clone(),
            auth_service: auth,
            email_verification_service: verification,
            auth_rate_limiter: limiter,
        },
        mail,
    )
}

async fn create_account(pool: &PgPool, email: &str) -> Uuid {
    let hash = Argon2PasswordHasher
        .hash_password("OldPassword123!")
        .unwrap();
    let id: Uuid = sqlx::query_scalar(
        r#"
        INSERT INTO auth.accounts (email, email_verified_at)
        VALUES ($1, clock_timestamp())
        RETURNING id
        "#,
    )
    .bind(email)
    .fetch_one(pool)
    .await
    .unwrap();
    sqlx::query(
        r#"
        INSERT INTO auth.password_credentials (account_id, password_hash)
        VALUES ($1, $2)
        "#,
    )
    .bind(id)
    .bind(hash)
    .execute(pool)
    .await
    .unwrap();
    id
}

async fn call(
    test_router: &axum::Router,
    path: &str,
    body: serde_json::Value,
    access: Option<&str>,
    expected: StatusCode,
) -> serde_json::Value {
    let mut request = Request::builder()
        .method("POST")
        .uri(path)
        .header("content-type", "application/json")
        .extension(ConnectInfo(
            "192.0.2.25:9000".parse::<std::net::SocketAddr>().unwrap(),
        ));
    if let Some(token) = access {
        request = request.header("authorization", format!("Bearer {token}"));
    }
    let response = test_router
        .clone()
        .oneshot(request.body(Body::from(body.to_string())).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), expected);
    assert_eq!(response.headers()["cache-control"], "no-store");
    assert_eq!(response.headers()["pragma"], "no-cache");
    let bytes = to_bytes(response.into_body(), 8192).await.unwrap();
    if bytes.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    }
}

fn reset(id: Uuid, code: String, password: &str) -> ResetPassword {
    ResetPassword {
        recovery_attempt_id: id,
        code,
        new_password: password.into(),
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn change_revokes_both_clients_and_blocks_old_refresh(pool: PgPool) {
    let (state, _) = fixture(&pool);
    let id = create_account(&pool, "change@example.invalid").await;
    let test_router = crate::routes::router(state.clone());
    let mut tokens = Vec::new();
    for client in ["desktop", "mobile"] {
        tokens.push(call(&test_router, "/api/v1/auth/login", serde_json::json!({"email":"change@example.invalid","password":"OldPassword123!","client_type":client}), None, StatusCode::OK).await);
    }
    let token = tokens[0]["access_token"].as_str().unwrap();
    let bad = call(
        &test_router,
        "/api/v1/auth/change-password",
        serde_json::json!({"current_password":"wrong","new_password":"NewPassword456!"}),
        Some(token),
        StatusCode::BAD_REQUEST,
    )
    .await;
    assert_eq!(bad["type"], "/problems/invalid-current-password");
    call(
        &test_router,
        "/api/v1/auth/change-password",
        serde_json::json!({"current_password":"OldPassword123!","new_password":"NewPassword456!"}),
        Some(token),
        StatusCode::NO_CONTENT,
    )
    .await;
    let count: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM auth.sessions
        WHERE account_id = $1
        AND revoked_at IS NULL
        "#,
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
    for t in &tokens {
        call(
            &test_router,
            "/api/v1/auth/refresh",
            serde_json::json!({"refresh_token":t["refresh_token"]}),
            None,
            StatusCode::UNAUTHORIZED,
        )
        .await;
    }
    call(&test_router, "/api/v1/auth/change-password", serde_json::json!({"current_password":"NewPassword456!","new_password":"ThirdPassword789!"}), Some(token), StatusCode::UNAUTHORIZED).await;
    call(&test_router, "/api/v1/auth/login", serde_json::json!({"email":"change@example.invalid","password":"NewPassword456!","client_type":"desktop"}), None, StatusCode::OK).await;
    let events: i64 = sqlx::query_scalar(
        r#"
        SELECT count(*)
        FROM auth.session_revocation_outbox
        WHERE event->'payload'->>'reason' = 'password_changed'
        "#,
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(events, 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn recovery_replaces_old_code_but_cooldown_preserves_it(pool: PgPool) {
    let (state, mail) = fixture(&pool);
    let id = create_account(&pool, "reset@example.invalid").await;
    let service = state.password_service.as_ref().unwrap();
    let first = service
        .forgot("reset@example.invalid".into())
        .await
        .unwrap();
    let first_code = mail.code().await;
    let suppressed = service
        .forgot("reset@example.invalid".into())
        .await
        .unwrap();
    assert!(crate::repositories::passwords_repository::recovery(
        &pool,
        suppressed.recovery_attempt_id
    )
    .await
    .unwrap()
    .is_none());
    assert!(
        crate::repositories::passwords_repository::recovery(&pool, first.recovery_attempt_id)
            .await
            .unwrap()
            .unwrap()
            .usable
    );
    sqlx::query(
        r#"
        UPDATE auth.password_recovery_attempts
        SET created_at = created_at - interval '61 seconds'
        WHERE account_id = $1
        "#,
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    mail.messages.lock().unwrap().clear();
    let second = service
        .forgot("reset@example.invalid".into())
        .await
        .unwrap();
    let second_code = mail.code().await;
    assert!(matches!(
        service
            .reset(reset(
                first.recovery_attempt_id,
                first_code,
                "NewPassword456!"
            ))
            .await,
        Err(PasswordError::InvalidCode)
    ));
    assert!(matches!(
        service
            .reset(reset(
                second.recovery_attempt_id,
                second_code.clone(),
                "short"
            ))
            .await,
        Err(PasswordError::InvalidPassword)
    ));
    assert!(matches!(
        service
            .reset(reset(
                second.recovery_attempt_id,
                second_code.clone(),
                "OldPassword123!"
            ))
            .await,
        Err(PasswordError::Unchanged)
    ));
    service
        .reset(reset(
            second.recovery_attempt_id,
            second_code.clone(),
            "NewPassword456!",
        ))
        .await
        .unwrap();
    assert!(matches!(
        service
            .reset(reset(
                second.recovery_attempt_id,
                second_code,
                "ThirdPassword789!"
            ))
            .await,
        Err(PasswordError::InvalidCode)
    ));
    let hash: String = sqlx::query_scalar(
        r#"
        SELECT password_hash
        FROM auth.password_credentials
        WHERE account_id = $1
        "#,
    )
    .bind(id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert!(Argon2PasswordHasher
        .verify_password("NewPassword456!", &hash)
        .unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn invalid_codes_persist_five_failures_even_with_invalid_new_password(pool: PgPool) {
    let (state, mail) = fixture(&pool);
    create_account(&pool, "failures@example.invalid").await;
    let service = state.password_service.as_ref().unwrap();
    let attempt = service
        .forgot("failures@example.invalid".into())
        .await
        .unwrap();
    let code = mail.code().await;
    for _ in 0..5 {
        assert!(matches!(
            service
                .reset(reset(
                    attempt.recovery_attempt_id,
                    "bad-format".into(),
                    "short"
                ))
                .await,
            Err(PasswordError::InvalidCode)
        ));
    }
    let count: i32 = sqlx::query_scalar(
        r#"
        SELECT failed_attempts
        FROM auth.password_recovery_attempts
        WHERE id = $1
        "#,
    )
    .bind(attempt.recovery_attempt_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 5);
    assert!(matches!(
        service
            .reset(reset(attempt.recovery_attempt_id, code, "NewPassword456!"))
            .await,
        Err(PasswordError::InvalidCode)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn ineligible_accounts_have_generic_acceptance_without_real_attempts(pool: PgPool) {
    let (state, mail) = fixture(&pool);
    for email in [
        "blocked@example.invalid",
        "unverified@example.invalid",
        "google@example.invalid",
    ] {
        let id = create_account(&pool, email).await;
        if email.starts_with("blocked") {
            sqlx::query("UPDATE auth.accounts SET status='locked' WHERE id=$1")
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        }
        if email.starts_with("unverified") {
            sqlx::query("UPDATE auth.accounts SET email=NULL,email_verified_at=NULL WHERE id=$1")
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        }
        if email.starts_with("google") {
            sqlx::query("DELETE FROM auth.password_credentials WHERE account_id=$1")
                .bind(id)
                .execute(&pool)
                .await
                .unwrap();
        }
    }
    let test_router = crate::routes::router(state.clone());
    for email in [
        "blocked@example.invalid",
        "unverified@example.invalid",
        "google@example.invalid",
        "unknown@example.invalid",
    ] {
        let b = call(
            &test_router,
            "/api/v1/auth/forgot-password",
            serde_json::json!({"email":email}),
            None,
            StatusCode::ACCEPTED,
        )
        .await;
        assert_eq!(b.as_object().unwrap().len(), 3);
        call(&test_router, "/api/v1/auth/reset-password", serde_json::json!({"recovery_attempt_id":b["recovery_attempt_id"],"code":"12345678","new_password":"NewPassword456!"}), None, StatusCode::BAD_REQUEST).await;
    }
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth.password_recovery_attempts")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
    assert!(mail.messages.lock().unwrap().is_empty());
}

#[sqlx::test(migrations = "./migrations")]
async fn simultaneous_correct_resets_have_only_one_winner(pool: PgPool) {
    let (state, mail) = fixture(&pool);
    create_account(&pool, "race@example.invalid").await;
    let service = state.password_service.as_ref().unwrap();
    let a = service.forgot("race@example.invalid".into()).await.unwrap();
    let code = mail.code().await;
    let (left, right) = tokio::join!(
        service.reset(reset(
            a.recovery_attempt_id,
            code.clone(),
            "NewPassword456!"
        )),
        service.reset(reset(a.recovery_attempt_id, code, "ThirdPassword789!"))
    );
    assert_eq!(usize::from(left.is_ok()) + usize::from(right.is_ok()), 1);
    for outcome in [&left, &right] {
        assert!(matches!(
            outcome,
            Ok(()) | Err(PasswordError::InvalidCode | PasswordError::Changed)
        ));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn expired_and_stale_credential_attempts_are_rejected_and_cleaned(pool: PgPool) {
    let (state, mail) = fixture(&pool);
    let id = create_account(&pool, "expiry@example.invalid").await;
    let service = state.password_service.as_ref().unwrap();
    let a = service
        .forgot("expiry@example.invalid".into())
        .await
        .unwrap();
    let code = mail.code().await;
    sqlx::query(
        r#"
        UPDATE auth.password_credentials
        SET password_changed_at = password_changed_at + interval '1 second'
        WHERE account_id = $1
        "#,
    )
    .bind(id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        service
            .reset(reset(
                a.recovery_attempt_id,
                code.clone(),
                "NewPassword456!"
            ))
            .await,
        Err(PasswordError::InvalidCode)
    ));
    sqlx::query(
        r#"
        UPDATE auth.password_recovery_attempts
        SET expires_at = clock_timestamp() - interval '25 hours', created_at = clock_timestamp() - interval '26 hours'
        WHERE id = $1
        "#,
    )
    .bind(a.recovery_attempt_id)
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        service
            .reset(reset(a.recovery_attempt_id, code, "NewPassword456!"))
            .await,
        Err(PasswordError::InvalidCode)
    ));
    assert_eq!(service.cleanup().await.unwrap(), 1);
}

#[test]
fn recovery_template_escapes_code_and_has_no_links_or_internal_metadata() {
    for locale in ["es-MX", "en"] {
        let b = crate::email::recovery_template::recovery_email_content(locale, "<12345678>");
        assert!(b.html.contains("&lt;12345678&gt;"));
        assert!(!b.html.contains("href="));
        assert!(!b.html.contains("src="));
        assert!(!b.text.contains("timezone"));
        assert!(b.text.contains("10"));
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn reset_rollback_preserves_code_password_and_sessions(pool: PgPool) {
    let (state, mail) = fixture(&pool);
    let account = create_account(&pool, "rollback@example.invalid").await;
    sqlx::query("SELECT auth.open_session($1,'desktop',interval '20 days')")
        .bind(account)
        .execute(&pool)
        .await
        .unwrap();
    let service = state.password_service.as_ref().unwrap();
    let attempt = service
        .forgot("rollback@example.invalid".into())
        .await
        .unwrap();
    let code = mail.code().await;
    sqlx::raw_sql(
        r#"
        CREATE FUNCTION auth.reject_test_password() RETURNS trigger
        LANGUAGE plpgsql AS $$
        BEGIN
            RAISE EXCEPTION 'test-only failure';
        END
        $$;

        CREATE TRIGGER reject_test_password
        BEFORE UPDATE ON auth.password_credentials
        FOR EACH ROW EXECUTE FUNCTION auth.reject_test_password();
        "#,
    )
    .execute(&pool)
    .await
    .unwrap();
    assert!(matches!(
        service
            .reset(reset(
                attempt.recovery_attempt_id,
                code.clone(),
                "NewPassword456!"
            ))
            .await,
        Err(PasswordError::Internal)
    ));
    assert!(
        crate::repositories::passwords_repository::recovery(&pool, attempt.recovery_attempt_id)
            .await
            .unwrap()
            .unwrap()
            .usable
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth.sessions WHERE revoked_at IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap(),
        1
    );
    sqlx::query("DROP TRIGGER reject_test_password ON auth.password_credentials")
        .execute(&pool)
        .await
        .unwrap();
    service
        .reset(reset(attempt.recovery_attempt_id, code, "NewPassword456!"))
        .await
        .unwrap();
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM auth.sessions WHERE revoked_at IS NULL")
            .fetch_one(&pool)
            .await
            .unwrap(),
        0
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_forgot_only_creates_one_pending_attempt(pool: PgPool) {
    let (state, _) = fixture(&pool);
    create_account(&pool, "forgot-race@example.invalid").await;
    let service = state.password_service.as_ref().unwrap();
    let (a, b) = tokio::join!(
        service.forgot("forgot-race@example.invalid".into()),
        service.forgot("forgot-race@example.invalid".into())
    );
    assert_ne!(
        a.unwrap().recovery_attempt_id,
        b.unwrap().recovery_attempt_id
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>(
            r#"
        SELECT count(*)
        FROM auth.password_recovery_attempts
        WHERE invalidated_at IS NULL
        AND consumed_at IS NULL
        "#,
        )
        .fetch_one(&pool)
        .await
        .unwrap(),
        1
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn http_rejects_bad_access_extra_fields_and_oversized_body(pool: PgPool) {
    let (state, _) = fixture(&pool);
    let test_router = crate::routes::router(state);
    call(
        &test_router,
        "/api/v1/auth/change-password",
        serde_json::json!({"current_password":"OldPassword123!","new_password":"NewPassword456!"}),
        Some("svc_not-an-access-token"),
        StatusCode::UNAUTHORIZED,
    )
    .await;
    call(
        &test_router,
        "/api/v1/auth/forgot-password",
        serde_json::json!({"email":"person@example.invalid","extra":true}),
        None,
        StatusCode::BAD_REQUEST,
    )
    .await;
    call(&test_router, "/api/v1/auth/reset-password", serde_json::json!({"recovery_attempt_id":"not-a-uuid","code":"12345678","new_password":"NewPassword456!"}), None, StatusCode::BAD_REQUEST).await;
    call(
        &test_router,
        "/api/v1/auth/forgot-password",
        serde_json::json!({"email":"x".repeat(5000)}),
        None,
        StatusCode::PAYLOAD_TOO_LARGE,
    )
    .await;
}

#[sqlx::test(migrations = "./migrations")]
async fn redis_failure_is_closed_for_all_password_operations(pool: PgPool) {
    let (mut state, mail) = fixture(&pool);
    let limiter = Arc::new(AuthRateLimiter::for_tests("redis://127.0.0.1:1/0".into()));
    state.password_service = Some(
        PasswordService::new(
            crate::services::password_service::PasswordServiceDependencies {
                pool: pool.clone(),
                passwords: state.auth_service.password_processor(),
                sender: mail,
                limiter: limiter.clone(),
            },
            &[0x22; 32],
        )
        .unwrap(),
    );
    state.auth_rate_limiter = limiter;
    let test_router = crate::routes::router(state);
    for path in ["change-password", "forgot-password", "reset-password"] {
        call(
            &test_router,
            &format!("/api/v1/auth/{path}"),
            serde_json::json!({}),
            None,
            StatusCode::SERVICE_UNAVAILABLE,
        )
        .await;
    }
}

#[sqlx::test(migrations = "./migrations")]
async fn reset_racing_refresh_never_keeps_a_refresh_alive(pool: PgPool) {
    let (state, mail) = fixture(&pool);
    create_account(&pool, "refresh-race@example.invalid").await;
    let sessions = state.sessions();
    let tokens = sessions
        .login(crate::dto::sessions::LoginRequest {
            email: "refresh-race@example.invalid".into(),
            password: "OldPassword123!".into(),
            client_type: crate::dto::sessions::ClientType::Desktop,
            device_name: None,
        })
        .await
        .unwrap();
    let service = state.password_service.as_ref().unwrap();
    let attempt = service
        .forgot("refresh-race@example.invalid".into())
        .await
        .unwrap();
    let code = mail.code().await;
    let (changed, refreshed) = tokio::join!(
        service.reset(reset(attempt.recovery_attempt_id, code, "NewPassword456!")),
        sessions.refresh(&tokens.refresh_token)
    );
    changed.unwrap();
    assert!(matches!(
        &refreshed,
        Ok(_) | Err(crate::services::session_service::SessionError::InvalidRefresh)
    ));
    if let Ok(next) = refreshed {
        assert!(matches!(
            sessions.refresh(&next.refresh_token).await,
            Err(crate::services::session_service::SessionError::InvalidRefresh)
        ));
    }
    assert!(matches!(
        sessions.refresh(&tokens.refresh_token).await,
        Err(crate::services::session_service::SessionError::InvalidRefresh)
    ));
}

#[tokio::test]
#[ignore = "requires the isolated Redis test service"]
async fn change_account_budget_is_shared_and_separate_from_login() {
    let url = std::env::var("REDIS_URL").unwrap();
    let first = AuthRateLimiter::for_tests(url.clone());
    let second = AuthRateLimiter::for_tests(url);
    let account = Uuid::new_v4();
    for _ in 0..5 {
        assert_eq!(
            first.check_change_account(account).await.unwrap(),
            crate::protection::rate_limit::RateLimitDecision::Allowed
        );
    }
    assert!(matches!(
        second.check_change_account(account).await.unwrap(),
        crate::protection::rate_limit::RateLimitDecision::Limited { .. }
    ));
    assert_eq!(
        second
            .check_login_account(&format!("{account}@example.invalid"))
            .await
            .unwrap(),
        crate::protection::rate_limit::RateLimitDecision::Allowed
    );
}

#[sqlx::test(migrations = "./migrations")]
#[ignore = "requires the isolated Mailpit test service"]
async fn recovery_mailpit_delivery_uses_approved_template_and_completes_reset(pool: PgPool) {
    let (state, _) = fixture(&pool);
    let email = format!("password-{}@example.invalid", Uuid::new_v4());
    create_account(&pool, &email).await;
    let sender = crate::providers::mailpit::MailpitEmailSender::new(
        "127.0.0.1".into(),
        std::env::var("AUTH_TEST_MAILPIT_SMTP_PORT")
            .unwrap()
            .parse()
            .unwrap(),
        "Sonimbus <test@example.invalid>".into(),
    )
    .unwrap();
    let service = PasswordService::new(
        crate::services::password_service::PasswordServiceDependencies {
            pool: pool.clone(),
            passwords: state.auth_service.password_processor(),
            sender: Arc::new(sender),
            limiter: state.auth_rate_limiter,
        },
        &[0x33; 32],
    )
    .unwrap();
    let attempt = service.forgot(email.clone()).await.unwrap();
    let message = super::support::mailpit_message(&email).await;
    let text = message["Text"].as_str().unwrap();
    assert!(text.contains("El código vence en 10 minutos y solo puede usarse una vez."));
    assert!(!message["HTML"].as_str().unwrap().contains("href="));
    let code = text
        .split_whitespace()
        .find(|word| word.len() == 8 && word.bytes().all(|b| b.is_ascii_digit()))
        .expect("Missing code");
    service
        .reset(reset(
            attempt.recovery_attempt_id,
            code.into(),
            "RecoveredPass456!",
        ))
        .await
        .unwrap();
}
