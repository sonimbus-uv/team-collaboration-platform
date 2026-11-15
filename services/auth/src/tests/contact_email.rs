use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::{
    dto::{contact_email::ContactVerificationAccepted, sessions::TokenResponse},
    ports::email_verification::{EmailDeliveryError, VerificationEmailSender},
    repositories::{
        contact_email_repository::ContactEmailRepository,
        session_authentication_repository::SessionActor, sessions_repository::SessionsRepository,
    },
    services::contact_email_service::{
        ContactEmailError, ContactEmailService, ContactRequestResult,
    },
};

#[derive(Default)]
struct FakeContactEmailSender {
    sent: Mutex<Vec<(String, String)>>,
    fail: AtomicBool,
}

#[async_trait::async_trait]
impl VerificationEmailSender for FakeContactEmailSender {
    async fn send(&self, _: &str, _: &str, _: &str) -> Result<(), EmailDeliveryError> {
        panic!("Contact verification must not send registration mail");
    }

    async fn send_contact_verification(
        &self,
        recipient: &str,
        code: &str,
        _: &str,
    ) -> Result<(), EmailDeliveryError> {
        self.sent
            .lock()
            .unwrap()
            .push((recipient.into(), code.into()));
        if self.fail.load(Ordering::SeqCst) {
            Err(EmailDeliveryError::Transport)
        } else {
            Ok(())
        }
    }
}

impl FakeContactEmailSender {
    fn code(&self) -> String {
        self.sent.lock().unwrap().last().unwrap().1.clone()
    }
}

async fn contact_fixture(
    pool: &PgPool,
) -> (
    crate::state::AppState,
    TokenResponse,
    Arc<FakeContactEmailSender>,
) {
    let account: Uuid = sqlx::query_scalar("INSERT INTO auth.accounts DEFAULT VALUES RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap();
    let identity: Uuid = sqlx::query_scalar("INSERT INTO auth.oauth_identities(account_id, provider, provider_subject) VALUES ($1, 'google', $2) RETURNING id")
        .bind(account)
        .bind(Uuid::new_v4().to_string())
        .fetch_one(pool)
        .await
        .unwrap();
    let sid: Uuid = sqlx::query_scalar("SELECT auth.open_authenticated_session($1, 'desktop', interval '20 days', ROW('google', $2, NULL, clock_timestamp())::auth.session_authentication_context)")
        .bind(account)
        .bind(identity)
        .fetch_one(pool)
        .await
        .unwrap();
    let mut state = super::sessions::test_state(pool, false);
    let mut tx = pool.begin().await.unwrap();
    let session = SessionsRepository::lock(&mut tx, sid).await.unwrap();
    let tokens = state
        .sessions()
        .issue_tokens(&mut tx, &session)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    let mail = Arc::new(FakeContactEmailSender::default());
    state.contact_email_service = Some(
        ContactEmailService::new(
            ContactEmailRepository { pool: pool.clone() },
            mail.clone(),
            state.auth_rate_limiter.clone(),
            &[0x53; 32],
        )
        .unwrap(),
    );
    (state, tokens, mail)
}

fn session_actor(tokens: &TokenResponse) -> SessionActor {
    SessionActor {
        account_id: tokens.account_id,
        session_id: tokens.sid,
    }
}

async fn start_verification(
    service: &ContactEmailService,
    tokens: &TokenResponse,
    email: &str,
) -> ContactVerificationAccepted {
    let result = service
        .start(session_actor(tokens), email.into())
        .await
        .unwrap_or_else(|_| panic!("Contact attempt creation failed"));
    match result {
        ContactRequestResult::Accepted(attempt) => attempt,
        ContactRequestResult::AlreadyVerified => panic!("Expected an unverified contact"),
    }
}

async fn send_contact_request(
    app: &Router,
    tokens: &TokenResponse,
    path: &str,
    body: Option<Value>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut request = Request::builder()
        .method(if body.is_some() { "POST" } else { "GET" })
        .uri(path)
        .header(
            header::AUTHORIZATION,
            format!("Bearer {}", tokens.access_token),
        )
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            body.map(|body| body.to_string()).unwrap_or_default(),
        ))
        .unwrap();
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        "192.0.2.77:45000".parse::<std::net::SocketAddr>().unwrap(),
    ));
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let body = to_bytes(response.into_body(), 8192).await.unwrap();
    let body = if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&body).unwrap()
    };
    (status, headers, body)
}

#[sqlx::test(migrations = "./migrations")]
async fn contact_http_flow_is_optional_idempotent_and_does_not_create_credentials_or_sessions(
    pool: PgPool,
) {
    let (state, tokens, mail) = contact_fixture(&pool).await;
    let authenticated_at: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>> =
        sqlx::query_scalar("SELECT last_authenticated_at FROM auth.sessions WHERE id = $1")
            .bind(tokens.sid)
            .fetch_one(&pool)
            .await
            .unwrap();
    let app = crate::routes::router(state.clone());
    let (_, _, initial) =
        send_contact_request(&app, &tokens, "/api/v1/auth/contact-email", None).await;
    assert!(initial["contact_email"].is_null());
    let (status, headers, attempt) = send_contact_request(
        &app,
        &tokens,
        "/api/v1/auth/contact-email/verification-attempts",
        Some(json!({"email":"  Owner+Tag@Example.Invalid  "})),
    )
    .await;
    assert_eq!(status, StatusCode::ACCEPTED);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    let body =
        json!({"verification_attempt_id":attempt["verification_attempt_id"],"code":mail.code()});
    for _ in 0..2 {
        assert_eq!(
            send_contact_request(
                &app,
                &tokens,
                "/api/v1/auth/contact-email/verify",
                Some(body.clone())
            )
            .await
            .0,
            StatusCode::NO_CONTENT
        );
    }
    let (_, _, verified) =
        send_contact_request(&app, &tokens, "/api/v1/auth/contact-email", None).await;
    assert_eq!(verified["contact_email"], "owner+tag@example.invalid");
    assert_eq!(verified["contact_email_verified"], true);
    assert!(verified["pending_contact_email"].is_null());
    assert_eq!(
        send_contact_request(
            &app,
            &tokens,
            "/api/v1/auth/contact-email/verification-attempts",
            Some(json!({"email":"owner+tag@example.invalid"}))
        )
        .await
        .0,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        send_contact_request(
            &app,
            &tokens,
            "/api/v1/auth/contact-email/verification-attempts",
            Some(json!({"email":"different@example.invalid"}))
        )
        .await
        .0,
        StatusCode::CONFLICT
    );
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM auth.password_credentials), (SELECT count(*) FROM auth.sessions)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(counts, (0, 1));
    let after: Option<sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>> =
        sqlx::query_scalar("SELECT last_authenticated_at FROM auth.sessions WHERE id = $1")
            .bind(tokens.sid)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(authenticated_at, after);

    assert!(state
        .sessions()
        .refresh(&tokens.refresh_token)
        .await
        .is_ok());
}

#[sqlx::test(migrations = "./migrations")]
async fn replacement_invalidates_codes_and_same_proposal_does_not_resend_or_extend_deadline(
    pool: PgPool,
) {
    let (state, tokens, mail) = contact_fixture(&pool).await;
    let service = state.contact_email_service.as_ref().unwrap();
    let first = start_verification(service, &tokens, "first@example.invalid").await;
    let old_code = mail.code();
    let duplicate = start_verification(service, &tokens, "FIRST@example.invalid").await;
    assert_eq!(
        first.verification_attempt_id,
        duplicate.verification_attempt_id
    );
    assert_eq!(first.expires_at, duplicate.expires_at);
    assert_eq!(mail.sent.lock().unwrap().len(), 1);
    let second = start_verification(service, &tokens, "second@example.invalid").await;
    assert!(matches!(
        service
            .verify(
                session_actor(&tokens),
                first.verification_attempt_id,
                old_code
            )
            .await,
        Err(ContactEmailError::InvalidCode)
    ));
    assert!(service
        .verify(
            session_actor(&tokens),
            second.verification_attempt_id,
            mail.code()
        )
        .await
        .is_ok());
}

#[sqlx::test(migrations = "./migrations")]
async fn malformed_codes_persist_five_failures_without_touching_another_accounts_attempt(
    pool: PgPool,
) {
    let (state, tokens, mail) = contact_fixture(&pool).await;
    let service = state.contact_email_service.as_ref().unwrap();
    let attempt = start_verification(service, &tokens, "owner@example.invalid").await;
    let code = mail.code();
    let (_, other, _) = contact_fixture(&pool).await;
    assert!(matches!(
        service
            .verify(
                session_actor(&other),
                attempt.verification_attempt_id,
                "bad".into()
            )
            .await,
        Err(ContactEmailError::InvalidCode)
    ));
    for value in ["", "short", "123456789", "abcdefgh", "1234\n678"] {
        assert!(matches!(
            service
                .verify(
                    session_actor(&tokens),
                    attempt.verification_attempt_id,
                    value.into()
                )
                .await,
            Err(ContactEmailError::InvalidCode)
        ));
    }
    assert!(matches!(
        service
            .verify(
                session_actor(&tokens),
                attempt.verification_attempt_id,
                code
            )
            .await,
        Err(ContactEmailError::InvalidCode)
    ));
    let failures: i32 = sqlx::query_scalar(
        "SELECT failed_attempts FROM auth.contact_email_verification_attempts WHERE id = $1",
    )
    .bind(attempt.verification_attempt_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(failures, 5);
}

#[sqlx::test(migrations = "./migrations")]
async fn confirmation_racing_password_registration_has_one_verified_owner(pool: PgPool) {
    let (state, tokens, mail) = contact_fixture(&pool).await;
    let service = state.contact_email_service.as_ref().unwrap();
    let attempt = start_verification(service, &tokens, "race@example.invalid").await;
    let repository =
        crate::repositories::registration_attempts_repository::RegistrationAttemptsRepository::new(
            pool.clone(),
        );
    let key = crate::security::registration_code::RegistrationCodeKey::new(&[0x18; 32]).unwrap();
    let id = Uuid::new_v4();
    repository
        .create(
            "race@example.invalid",
            "test-password-hash",
            id,
            &key.digest(id, 0, "00123456"),
        )
        .await
        .unwrap();
    let (contact, registration) = tokio::join!(
        service.verify(
            session_actor(&tokens),
            attempt.verification_attempt_id,
            mail.code()
        ),
        repository.confirm(id, "00123456", &key)
    );
    assert_eq!(
        usize::from(contact.is_ok()) + usize::from(registration.is_ok()),
        1
    );
    if let Err(error) = contact {
        assert!(matches!(error, ContactEmailError::InUse));
    }
    let owners: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM auth.accounts WHERE email = 'RACE@example.invalid'",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(owners, 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn provider_failure_keeps_attempt_and_redis_failure_creates_no_new_state(pool: PgPool) {
    let (state, tokens, mail) = contact_fixture(&pool).await;
    let service = state.contact_email_service.as_ref().unwrap();
    mail.fail.store(true, Ordering::SeqCst);
    let attempt = start_verification(service, &tokens, "owner@example.invalid").await;
    assert!(service
        .verify(
            session_actor(&tokens),
            attempt.verification_attempt_id,
            mail.code()
        )
        .await
        .is_ok());
    let (_, other, _) = contact_fixture(&pool).await;
    let closed = ContactEmailService::new(
        ContactEmailRepository { pool: pool.clone() },
        mail.clone(),
        Arc::new(
            crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(
                "redis://127.0.0.1:1/0".into(),
            ),
        ),
        &[0x53; 32],
    )
    .unwrap();
    assert!(matches!(
        closed
            .start(session_actor(&other), "no-send@example.invalid".into())
            .await,
        Err(ContactEmailError::Unavailable)
    ));
    let count: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM auth.account_email_proposals WHERE account_id = $1",
    )
    .bind(other.account_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(count, 0);
}

async fn aged_attempt(pool: &PgPool, tokens: &TokenResponse, code_version: i32) -> Uuid {
    use crate::security::contact_email_code::{ContactCodeContext, ContactEmailCodeKey};
    sqlx::query("INSERT INTO auth.account_email_proposals(account_id, email) VALUES ($1, 'aged@example.invalid')")
        .bind(tokens.account_id)
        .execute(pool)
        .await
        .unwrap();
    let id = Uuid::new_v4();
    let key = ContactEmailCodeKey::new(&[0x53; 32]).unwrap();
    let mac = key.digest(
        &ContactCodeContext {
            account_id: tokens.account_id,
            email: "aged@example.invalid",
            attempt_id: id,
            contact_version: 1,
            code_version,
        },
        "00123456",
    );
    sqlx::query("WITH observed AS (SELECT clock_timestamp() AS instant) INSERT INTO auth.contact_email_verification_attempts(id, account_id, email, contact_email_version, code_mac, code_version, failed_attempts, created_at, last_sent_at, expires_at) SELECT $1, $2, 'aged@example.invalid', 1, $3, $4, 2, instant - interval '5 minutes', instant - interval '61 seconds', instant + interval '5 minutes' FROM observed")
        .bind(id)
        .bind(tokens.account_id)
        .bind(mac)
        .bind(code_version)
        .execute(pool)
        .await
        .unwrap();
    id
}

#[sqlx::test(migrations = "./migrations")]
async fn resend_preserves_deadline_and_failures_and_enforces_cooldown_and_three_replacements(
    pool: PgPool,
) {
    let (state, tokens, mail) = contact_fixture(&pool).await;
    let service = state.contact_email_service.as_ref().unwrap();
    let current = start_verification(service, &tokens, "current@example.invalid").await;
    assert!(matches!(
        service
            .resend(session_actor(&tokens), current.verification_attempt_id)
            .await,
        Err(ContactEmailError::Limited(_))
    ));
    let (_, aged, _) = contact_fixture(&pool).await;
    let id = aged_attempt(&pool, &aged, 0).await;
    let before: (i32, sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>) = sqlx::query_as("SELECT failed_attempts, expires_at FROM auth.contact_email_verification_attempts WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert!(service.resend(session_actor(&aged), id).await.is_ok());
    let after: (i32, sqlx::types::chrono::DateTime<sqlx::types::chrono::Utc>) = sqlx::query_as("SELECT failed_attempts, expires_at FROM auth.contact_email_verification_attempts WHERE id = $1")
        .bind(id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(before, after);
    assert!(matches!(
        service
            .verify(session_actor(&aged), id, "00123456".into())
            .await,
        Err(ContactEmailError::InvalidCode)
    ));
    assert!(service
        .verify(session_actor(&aged), id, mail.code())
        .await
        .is_ok());
    let (_, exhausted, _) = contact_fixture(&pool).await;
    let id = aged_attempt(&pool, &exhausted, 3).await;
    assert!(matches!(
        service.resend(session_actor(&exhausted), id).await,
        Err(ContactEmailError::Limited(_))
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_correct_confirmations_are_idempotent_and_assignment_rolls_back_on_failure(
    pool: PgPool,
) {
    let (state, tokens, mail) = contact_fixture(&pool).await;
    let service = state.contact_email_service.as_ref().unwrap();
    let attempt = start_verification(service, &tokens, "same@example.invalid").await;
    let code = mail.code();
    let (first, second) = tokio::join!(
        service.verify(
            session_actor(&tokens),
            attempt.verification_attempt_id,
            code.clone()
        ),
        service.verify(
            session_actor(&tokens),
            attempt.verification_attempt_id,
            code
        )
    );
    assert!(first.is_ok() && second.is_ok());
    let (_, other, _) = contact_fixture(&pool).await;
    let attempt = start_verification(service, &other, "rollback@example.invalid").await;
    sqlx::raw_sql("CREATE FUNCTION auth.reject_test_contact() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Injected contact consumption failure'; END; $$; CREATE TRIGGER reject_test_contact BEFORE UPDATE OF consumed_at ON auth.contact_email_verification_attempts FOR EACH ROW EXECUTE FUNCTION auth.reject_test_contact();")
        .execute(&pool)
        .await
        .unwrap();
    assert!(service
        .verify(
            session_actor(&other),
            attempt.verification_attempt_id,
            mail.code()
        )
        .await
        .is_err());
    assert!(service
        .snapshot(session_actor(&other))
        .await
        .unwrap_or_else(|_| panic!("Snapshot failed"))
        .contact_email
        .is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn mailpit_contact_code_uses_its_own_template_and_verifies_contact(pool: PgPool) {
    let (state, tokens, _) = contact_fixture(&pool).await;
    let port = std::env::var("AUTH_TEST_MAILPIT_SMTP_PORT")
        .unwrap()
        .parse()
        .unwrap();
    let sender = Arc::new(
        crate::providers::mailpit::MailpitEmailSender::new(
            "127.0.0.1".into(),
            port,
            "Sonimbus <noreply@example.invalid>".into(),
        )
        .unwrap(),
    );
    let service = ContactEmailService::new(
        ContactEmailRepository { pool: pool.clone() },
        sender,
        state.auth_rate_limiter.clone(),
        &[0x53; 32],
    )
    .unwrap();
    let email = format!("contact-{}@example.invalid", Uuid::new_v4());
    let attempt = start_verification(&service, &tokens, &email).await;
    let message = super::support::mailpit_message(&email).await;
    let code = message["Text"]
        .as_str()
        .unwrap()
        .split_whitespace()
        .find(|part| part.len() == 8 && part.bytes().all(|byte| byte.is_ascii_digit()))
        .unwrap();
    assert!(!message["HTML"].as_str().unwrap().contains("href="));
    assert!(service
        .verify(
            session_actor(&tokens),
            attempt.verification_attempt_id,
            code.into()
        )
        .await
        .is_ok());
}

#[sqlx::test(migrations = "./migrations")]
async fn contact_and_registration_share_the_same_distributed_recipient_budget(pool: PgPool) {
    let (state, tokens, mail) = contact_fixture(&pool).await;
    let redis = std::env::var("REDIS_URL").expect("Redis test requires REDIS_URL");
    let limiter = Arc::new(
        crate::protection::auth_rate_limiter::AuthRateLimiter::for_tests(redis)
            .with_recipient_policy_for_tests(
                crate::protection::rate_limit::BucketPolicy::new(1, 600).unwrap(),
            ),
    );
    let service = ContactEmailService::new(
        ContactEmailRepository { pool: pool.clone() },
        mail.clone(),
        limiter.clone(),
        &[0x53; 32],
    )
    .unwrap();
    let email = format!("shared-{}@example.invalid", Uuid::new_v4());
    start_verification(&service, &tokens, &email).await;
    start_verification(&service, &tokens, &email).await;

    let registration = state
        .email_verification_service
        .start(&email, "test-hash", limiter.as_ref())
        .await;
    assert!(matches!(
        registration,
        Err(crate::services::email_verification_service::VerificationError::Limited(_))
    ));
    assert_eq!(mail.sent.lock().unwrap().len(), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn smtp_wait_does_not_retain_account_locks_or_a_database_transaction(pool: PgPool) {
    struct GatedMail {
        entered: tokio::sync::Notify,
        release: tokio::sync::Notify,
    }
    #[async_trait::async_trait]
    impl VerificationEmailSender for GatedMail {
        async fn send(&self, _: &str, _: &str, _: &str) -> Result<(), EmailDeliveryError> {
            panic!("Unexpected registration mail");
        }
        async fn send_contact_verification(
            &self,
            _: &str,
            _: &str,
            _: &str,
        ) -> Result<(), EmailDeliveryError> {
            self.entered.notify_one();
            self.release.notified().await;
            Ok(())
        }
    }
    let (state, tokens, _) = contact_fixture(&pool).await;
    let mail = Arc::new(GatedMail {
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Notify::new(),
    });
    let service = ContactEmailService::new(
        ContactEmailRepository { pool: pool.clone() },
        mail.clone(),
        state.auth_rate_limiter.clone(),
        &[0x53; 32],
    )
    .unwrap();
    let actor = session_actor(&tokens);
    let task = tokio::spawn(async move {
        service
            .start(actor, "no-lock@example.invalid".into())
            .await
            .is_ok()
    });
    let _guard = super::support::AbortOnDrop::new(&task);
    tokio::time::timeout(std::time::Duration::from_secs(3), mail.entered.notified())
        .await
        .unwrap();
    let mut tx = pool.begin().await.unwrap();
    sqlx::query("SET LOCAL lock_timeout = '100ms'")
        .execute(&mut *tx)
        .await
        .unwrap();
    sqlx::query("SELECT id FROM auth.accounts WHERE id = $1 FOR UPDATE")
        .bind(tokens.account_id)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    mail.release.notify_one();
    assert!(task.await.unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn http_admission_rejects_missing_access_account_selection_and_oversized_bodies(
    pool: PgPool,
) {
    let (state, tokens, mail) = contact_fixture(&pool).await;
    let app = crate::routes::router(state);
    let request = Request::builder()
        .method("POST")
        .uri("/api/v1/auth/contact-email/verification-attempts")
        .header(header::CONTENT_TYPE, "application/json")
        .extension(axum::extract::ConnectInfo(
            "192.0.2.77:45000".parse::<std::net::SocketAddr>().unwrap(),
        ))
        .body(Body::from(
            json!({"email":"owner@example.invalid"}).to_string(),
        ))
        .unwrap();
    assert_eq!(
        app.clone().oneshot(request).await.unwrap().status(),
        StatusCode::UNAUTHORIZED
    );
    let (status, _, _) = send_contact_request(
        &app,
        &tokens,
        "/api/v1/auth/contact-email/verification-attempts",
        Some(json!({"email":"owner@example.invalid","account_id":tokens.account_id})),
    )
    .await;
    assert_eq!(status, StatusCode::BAD_REQUEST);
    let (status, headers, _) = send_contact_request(
        &app,
        &tokens,
        "/api/v1/auth/contact-email/verification-attempts",
        Some(json!({"email":"x".repeat(5000)})),
    )
    .await;
    assert_eq!(status, StatusCode::PAYLOAD_TOO_LARGE);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert!(mail.sent.lock().unwrap().is_empty());
    let disabled = crate::routes::router(super::sessions::test_state(&pool, false));
    assert_eq!(
        send_contact_request(&disabled, &tokens, "/api/v1/auth/contact-email", None)
            .await
            .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
}
