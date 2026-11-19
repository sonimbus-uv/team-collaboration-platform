use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

use auth_service::federated_identity::{
    AuthorizationCode, AuthorizationRequest, EmailOwnership, FederatedIdentityError,
    FederatedIdentityProvider, ProviderEmail, VerifiedIdentity,
};
use axum::{
    body::{to_bytes, Body},
    http::{header, Request, StatusCode},
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use reqwest::Url;
use serde_json::{json, Value};
use sqlx::PgPool;
use tower::ServiceExt;

use crate::{
    dto::{
        google::{CreateGoogleAttempt, ExchangeGoogleAttempt, GoogleAttemptResponse},
        sessions::ClientType,
    },
    repositories::federated_auth_repository::FederatedAuthRepository,
    services::{
        google_auth_config::GoogleAuthConfig,
        google_auth_service::{hash, GoogleAuthError, GoogleAuthService, GoogleCallback},
    },
};

const VERIFIER: &str = "test-verifier-0123456789abcdefghijklmnopqrstuvwxyz";

struct FakeGoogle {
    email: String,
    ownership: EmailOwnership,
    calls: AtomicUsize,
    nonce: Mutex<Option<[u8; 32]>>,
}

#[async_trait::async_trait]
impl FederatedIdentityProvider for FakeGoogle {
    fn authorization_url(
        &self,
        request: AuthorizationRequest<'_>,
    ) -> Result<String, FederatedIdentityError> {
        *self.nonce.lock().unwrap() = Some(hash(request.nonce));
        let mut url = Url::parse("https://accounts.google.com/authorize").unwrap();
        url.query_pairs_mut().append_pair("state", request.state);
        Ok(url.into())
    }

    async fn exchange_code(
        &self,
        request: AuthorizationCode<'_>,
    ) -> Result<VerifiedIdentity, FederatedIdentityError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        assert_eq!(
            Some(*request.expected_nonce_hash),
            *self.nonce.lock().unwrap()
        );
        let now = sqlx::types::chrono::Utc::now().timestamp() as u64;
        Ok(VerifiedIdentity {
            issuer: "https://accounts.google.com",
            subject: "verified-google-subject".into(),
            email: Some(ProviderEmail {
                address: self.email.clone(),
                ownership: self.ownership,
            }),
            issued_at: now,
            expires_at: now + 3600,
        })
    }
}

fn google_test_state(
    pool: &PgPool,
    ownership: EmailOwnership,
) -> (crate::state::AppState, Arc<FakeGoogle>) {
    let provider = Arc::new(FakeGoogle {
        email: "person@example.invalid".into(),
        ownership,
        calls: AtomicUsize::new(0),
        nonce: Mutex::new(None),
    });
    let config = GoogleAuthConfig::new(
        "test.apps.googleusercontent.com".into(),
        "test-only-secret".into(),
        "https://auth.example.invalid/api/v1/auth/google/callback",
        r#"{"desktop":{"client_type":"desktop","url":"sonimbus://auth/google"}}"#,
    )
    .unwrap();
    let service = GoogleAuthService::new(
        FederatedAuthRepository { pool: pool.clone() },
        super::sessions::service(pool),
        config,
    )
    .unwrap()
    .with_provider(provider.clone());
    let mut state = super::sessions::test_state(pool, false);
    state.google_service = Some(service);
    (state, provider)
}

fn create_request() -> CreateGoogleAttempt {
    CreateGoogleAttempt {
        client_type: ClientType::Desktop,
        device_name: Some("Test device".into()),
        redirect_target: "desktop".into(),
        code_challenge: URL_SAFE_NO_PAD.encode(hash(VERIFIER)),
    }
}

async fn verified_login_attempt(
    service: &GoogleAuthService,
) -> (GoogleAttemptResponse, ExchangeGoogleAttempt) {
    let attempt = service
        .create(create_request())
        .await
        .unwrap_or_else(|_| panic!("Attempt creation failed"));
    complete_browser_authorization(service, attempt).await
}

async fn complete_browser_authorization(
    service: &GoogleAuthService,
    attempt: GoogleAttemptResponse,
) -> (GoogleAttemptResponse, ExchangeGoogleAttempt) {
    let ticket = Url::parse(&attempt.authorization_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "ticket")
        .unwrap()
        .1
        .into_owned();
    let authorization = service
        .authorize(&ticket)
        .await
        .unwrap_or_else(|_| panic!("Authorization failed"));
    let callback = service
        .callback(GoogleCallback {
            state: &authorization.state,
            cookie_state: &authorization.state,
            code: Some("test-code"),
            denied: false,
        })
        .await
        .unwrap_or_else(|_| panic!("Callback failed"));
    let completion = callback
        .redirect
        .query_pairs()
        .find(|(key, _)| key == "completion_code")
        .unwrap()
        .1
        .into_owned();
    let exchange = ExchangeGoogleAttempt {
        attempt_id: attempt.attempt_id,
        completion_code: completion,
        code_verifier: VERIFIER.into(),
    };
    (attempt, exchange)
}

async fn send_google_request(
    app: &Router,
    request: Request<Body>,
) -> (StatusCode, axum::http::HeaderMap, Value) {
    let mut request = request;
    request.extensions_mut().insert(axum::extract::ConnectInfo(
        "192.0.2.1:45000".parse::<std::net::SocketAddr>().unwrap(),
    ));
    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let headers = response.headers().clone();
    let bytes = to_bytes(response.into_body(), 16384).await.unwrap();
    let body = if bytes.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(&bytes).unwrap()
    };
    (status, headers, body)
}

fn json_post_request(path: &str, body: Value) -> Request<Body> {
    Request::builder()
        .method("POST")
        .uri(path)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body.to_string()))
        .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn browser_login_requires_cookie_pkce_and_secret_and_supports_existing_session_contract(
    pool: PgPool,
) {
    let (state, provider) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let app = crate::routes::router(state.clone());
    let (status, _, attempt) = send_google_request(&app, json_post_request("/api/v1/auth/google/attempts", json!({"client_type":"desktop","redirect_target":"desktop","code_challenge":URL_SAFE_NO_PAD.encode(hash(VERIFIER))})))
        .await;
    assert_eq!(status, StatusCode::CREATED);
    let url = Url::parse(attempt["authorization_url"].as_str().unwrap()).unwrap();
    let path = format!("{}?{}", url.path(), url.query().unwrap());
    assert_eq!(
        send_google_request(
            &app,
            Request::builder()
                .method("HEAD")
                .uri(&path)
                .body(Body::empty())
                .unwrap()
        )
        .await
        .0,
        StatusCode::METHOD_NOT_ALLOWED
    );
    let (status, headers, _) = send_google_request(
        &app,
        Request::builder().uri(&path).body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FOUND);
    assert_eq!(headers[header::REFERRER_POLICY], "no-referrer");
    let cookie = headers[header::SET_COOKIE].to_str().unwrap();
    assert!(cookie.contains("Secure; HttpOnly; SameSite=Lax"));
    let cookie = cookie.split(';').next().unwrap().to_owned();
    let google_url = Url::parse(headers[header::LOCATION].to_str().unwrap()).unwrap();
    let oauth_state = google_url
        .query_pairs()
        .find(|(key, _)| key == "state")
        .unwrap()
        .1
        .into_owned();
    let callback = format!("/api/v1/auth/google/callback?state={oauth_state}&code=test-code");
    assert_eq!(
        send_google_request(
            &app,
            Request::builder()
                .uri(&callback)
                .body(Body::empty())
                .unwrap()
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    let (status, headers, _) = send_google_request(
        &app,
        Request::builder()
            .uri(&callback)
            .header(header::COOKIE, &cookie)
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::FOUND);
    assert!(headers[header::SET_COOKIE]
        .to_str()
        .unwrap()
        .contains("Max-Age=0"));
    let destination = Url::parse(headers[header::LOCATION].to_str().unwrap()).unwrap();
    assert_eq!(destination.scheme(), "sonimbus");
    let completion = destination
        .query_pairs()
        .find(|(key, _)| key == "completion_code")
        .unwrap()
        .1
        .into_owned();
    let body = json!({"attempt_id":attempt["attempt_id"],"completion_code":completion,"code_verifier":VERIFIER});
    assert_eq!(
        send_google_request(
            &app,
            json_post_request("/api/v1/auth/google/exchange", body.clone())
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let mut incorrect_pkce = body.clone();
    incorrect_pkce["code_verifier"] =
        json!("another-verifier-0123456789abcdefghijklmnopqrstuvwxyz");
    let mut bad = json_post_request("/api/v1/auth/google/exchange", incorrect_pkce);
    bad.headers_mut().insert(
        "x-auth-attempt-secret",
        attempt["attempt_secret"].as_str().unwrap().parse().unwrap(),
    );
    assert_eq!(
        send_google_request(&app, bad).await.0,
        StatusCode::UNAUTHORIZED
    );
    let mut request = json_post_request("/api/v1/auth/google/exchange", body.clone());
    request.headers_mut().insert(
        "x-auth-attempt-secret",
        attempt["attempt_secret"].as_str().unwrap().parse().unwrap(),
    );
    let (status, headers, tokens) = send_google_request(&app, request).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    let claims = state
        .token_issuer
        .verify_for_auth(tokens["access_token"].as_str().unwrap())
        .unwrap();
    assert_eq!(claims.sid.to_string(), tokens["sid"].as_str().unwrap());
    assert!(state
        .sessions()
        .refresh(tokens["refresh_token"].as_str().unwrap())
        .await
        .is_ok());
    let mut replay = json_post_request("/api/v1/auth/google/exchange", body);
    replay.headers_mut().insert(
        "x-auth-attempt-secret",
        attempt["attempt_secret"].as_str().unwrap().parse().unwrap(),
    );
    assert_eq!(
        send_google_request(&app, replay).await.0,
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn concurrent_exchange_commits_exactly_one_identity_session_and_refresh(pool: PgPool) {
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.unwrap();
    let (attempt, request) = verified_login_attempt(&service).await;
    let duplicate = ExchangeGoogleAttempt {
        attempt_id: request.attempt_id,
        completion_code: request.completion_code.clone(),
        code_verifier: request.code_verifier.clone(),
    };
    let (first, second) = tokio::join!(
        service.exchange(request, &attempt.attempt_secret),
        service.exchange(duplicate, &attempt.attempt_secret)
    );
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM auth.oauth_identities), (SELECT count(*) FROM auth.sessions), (SELECT count(*) FROM auth.refresh_tokens)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(counts, (1, 1, 1));
}

#[sqlx::test(migrations = "./migrations")]
async fn callback_claim_prevents_duplicate_provider_exchange(pool: PgPool) {
    let (state, provider) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.unwrap();
    let attempt = service
        .create(create_request())
        .await
        .unwrap_or_else(|_| panic!("Creation failed"));
    let ticket = Url::parse(&attempt.authorization_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "ticket")
        .unwrap()
        .1
        .into_owned();
    let authorization = service
        .authorize(&ticket)
        .await
        .unwrap_or_else(|_| panic!("Authorization failed"));
    assert!(service.authorize(&ticket).await.is_err());
    let request = || GoogleCallback {
        state: &authorization.state,
        cookie_state: &authorization.state,
        code: Some("test-code"),
        denied: false,
    };
    let (first, second) = tokio::join!(service.callback(request()), service.callback(request()));
    assert_eq!(usize::from(first.is_ok()) + usize::from(second.is_ok()), 1);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 1);
}

#[sqlx::test(migrations = "./migrations")]
async fn external_email_does_not_link_to_its_existing_verified_owner(pool: PgPool) {
    let owner: uuid::Uuid = sqlx::query_scalar("INSERT INTO auth.accounts(email, email_verified_at) VALUES ('person@example.invalid', clock_timestamp()) RETURNING id")
        .fetch_one(&pool)
        .await
        .unwrap();
    let (state, _) = google_test_state(&pool, EmailOwnership::Unverified);
    let service = state.google_service.unwrap();
    let (attempt, request) = verified_login_attempt(&service).await;
    let tokens = service
        .exchange(request, &attempt.attempt_secret)
        .await
        .unwrap_or_else(|_| panic!("Exchange failed"));
    let crate::services::google_auth_service::GoogleExchangeResult::Tokens(tokens) = tokens else {
        panic!("Expected login tokens");
    };
    assert_ne!(tokens.account_id, owner);
    let contact: Option<String> =
        sqlx::query_scalar("SELECT email::text FROM auth.accounts WHERE id = $1")
            .bind(tokens.account_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(contact.is_none());
    let proposed: String = sqlx::query_scalar(
        "SELECT email::text FROM auth.account_email_proposals WHERE account_id = $1",
    )
    .bind(tokens.account_id)
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(proposed, "person@example.invalid");
}

#[sqlx::test(migrations = "./migrations")]
async fn authoritative_contact_conflict_requires_explicit_link_and_rolls_back(pool: PgPool) {
    sqlx::query("INSERT INTO auth.accounts(email, email_verified_at) VALUES ('PERSON@example.invalid', clock_timestamp())")
        .execute(&pool)
        .await
        .unwrap();
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.unwrap();
    let (attempt, request) = verified_login_attempt(&service).await;
    assert!(matches!(
        service.exchange(request, &attempt.attempt_secret).await,
        Err(GoogleAuthError::LinkRequired)
    ));
    let counts: (i64, i64) = sqlx::query_as(
        "SELECT (SELECT count(*) FROM auth.accounts), (SELECT count(*) FROM auth.oauth_identities)",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(counts, (1, 0));
}

#[sqlx::test(migrations = "./migrations")]
async fn refresh_insert_failure_rolls_back_identity_session_and_consumption(pool: PgPool) {
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.unwrap();
    let (attempt, request) = verified_login_attempt(&service).await;
    sqlx::raw_sql("CREATE FUNCTION auth.reject_test_refresh() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Injected refresh failure'; END; $$; CREATE TRIGGER reject_test_refresh BEFORE INSERT ON auth.refresh_tokens FOR EACH ROW EXECUTE FUNCTION auth.reject_test_refresh();")
        .execute(&pool)
        .await
        .unwrap();
    assert!(service
        .exchange(request, &attempt.attempt_secret)
        .await
        .is_err());
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM auth.accounts), (SELECT count(*) FROM auth.oauth_identities), (SELECT count(*) FROM auth.sessions)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(counts, (0, 0, 0));
    let status: String =
        sqlx::query_scalar("SELECT status FROM auth.federated_auth_attempts WHERE id = $1")
            .bind(attempt.attempt_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "verified");
}

#[sqlx::test(migrations = "./migrations")]
async fn linked_identity_of_a_blocked_account_cannot_open_a_session(pool: PgPool) {
    let account: uuid::Uuid =
        sqlx::query_scalar("INSERT INTO auth.accounts(status) VALUES ('locked') RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO auth.oauth_identities(account_id, provider, provider_subject) VALUES ($1, 'google', 'verified-google-subject')")
        .bind(account)
        .execute(&pool)
        .await
        .unwrap();
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.unwrap();
    let (attempt, request) = verified_login_attempt(&service).await;
    assert!(matches!(
        service.exchange(request, &attempt.attempt_secret).await,
        Err(GoogleAuthError::AccountBlocked)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.sessions")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn disabled_login_and_cancelled_attempts_never_create_a_google_session(pool: PgPool) {
    let disabled = crate::routes::router(super::sessions::test_state(&pool, false));
    assert_eq!(
        send_google_request(
            &disabled,
            json_post_request("/api/v1/auth/google/attempts", json!({}))
        )
        .await
        .0,
        StatusCode::SERVICE_UNAVAILABLE
    );
    let (state, provider) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.unwrap();
    let attempt = service
        .create(create_request())
        .await
        .unwrap_or_else(|_| panic!("Creation failed"));
    let ticket = Url::parse(&attempt.authorization_url)
        .unwrap()
        .query_pairs()
        .find(|(key, _)| key == "ticket")
        .unwrap()
        .1
        .into_owned();
    let browser = service
        .authorize(&ticket)
        .await
        .unwrap_or_else(|_| panic!("Authorization failed"));
    let result = service
        .callback(GoogleCallback {
            state: &browser.state,
            cookie_state: &browser.state,
            code: None,
            denied: true,
        })
        .await
        .unwrap_or_else(|_| panic!("Cancellation failed"));
    assert!(result
        .redirect
        .query_pairs()
        .any(|(key, value)| key == "error" && value == "cancelled"));
    assert_eq!(provider.calls.load(Ordering::SeqCst), 0);
    let status: String =
        sqlx::query_scalar("SELECT status FROM auth.federated_auth_attempts WHERE id = $1")
            .bind(attempt.attempt_id)
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(status, "cancelled");
}

async fn local_session(pool: &PgPool, fresh: bool) -> crate::dto::sessions::TokenResponse {
    let account: uuid::Uuid = sqlx::query_scalar("INSERT INTO auth.accounts(email, email_verified_at) VALUES ('owner@example.invalid', clock_timestamp()) RETURNING id")
        .fetch_one(pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO auth.password_credentials(account_id, password_hash) VALUES ($1, 'test-only-password-hash')")
        .bind(account)
        .execute(pool)
        .await
        .unwrap();
    let sid = sqlx::query_scalar("SELECT auth.open_session($1, 'desktop', interval '20 days')")
        .bind(account)
        .fetch_one(pool)
        .await
        .unwrap();
    let service = super::sessions::service(pool);
    let mut tx = pool.begin().await.unwrap();
    let session = crate::repositories::sessions_repository::SessionsRepository::lock(&mut tx, sid)
        .await
        .unwrap();
    let tokens = service.issue_tokens(&mut tx, &session).await.unwrap();
    tx.commit().await.unwrap();
    if fresh {
        service
            .reauthenticate(session_actor(&tokens), "TestPass123!".into())
            .await
            .unwrap();
    }
    tokens
}

fn session_actor(
    tokens: &crate::dto::sessions::TokenResponse,
) -> crate::repositories::session_authentication_repository::SessionActor {
    crate::repositories::session_authentication_repository::SessionActor {
        account_id: tokens.account_id,
        session_id: tokens.sid,
    }
}

fn link_request() -> crate::dto::google::LinkGoogleAttempt {
    crate::dto::google::LinkGoogleAttempt {
        redirect_target: "desktop".into(),
        code_challenge: URL_SAFE_NO_PAD.encode(hash(VERIFIER)),
    }
}

async fn verified_link_attempt(
    service: &GoogleAuthService,
    tokens: &crate::dto::sessions::TokenResponse,
) -> (GoogleAttemptResponse, ExchangeGoogleAttempt) {
    let attempt = service
        .create_link(session_actor(tokens), link_request())
        .await
        .unwrap_or_else(|_| panic!("Link attempt creation failed"));
    complete_browser_authorization(service, attempt).await
}

#[sqlx::test(migrations = "./migrations")]
async fn authenticated_link_is_idempotent_and_does_not_change_contact_password_or_session(
    pool: PgPool,
) {
    use crate::services::google_auth_service::GoogleExchangeResult;
    let tokens = local_session(&pool, true).await;
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.as_ref().unwrap();
    for _ in 0..2 {
        let (attempt, proof) = verified_link_attempt(service, &tokens).await;
        assert!(matches!(
            service.exchange(proof, &attempt.attempt_secret).await,
            Ok(GoogleExchangeResult::Linked)
        ));
    }
    let counts: (i64, i64, i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM auth.accounts), (SELECT count(*) FROM auth.sessions), (SELECT count(*) FROM auth.refresh_tokens), (SELECT count(*) FROM auth.account_security_events), (SELECT count(*) FROM auth.security_notification_outbox)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(counts, (1, 1, 1, 1, 1));
    let recipient: String =
        sqlx::query_scalar("SELECT recipient_email::text FROM auth.security_notification_outbox")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(recipient, "owner@example.invalid");
    let contact: String = sqlx::query_scalar("SELECT email::text FROM auth.accounts WHERE id = $1")
        .bind(tokens.account_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(contact, recipient);
    assert!(state
        .sessions()
        .refresh(&tokens.refresh_token)
        .await
        .is_ok());
    let app = crate::routes::router(state);
    let (status, _, connections) = send_google_request(
        &app,
        Request::builder()
            .uri("/api/v1/auth/connections")
            .header(
                header::AUTHORIZATION,
                format!("Bearer {}", tokens.access_token),
            )
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(connections["connections"][0]["provider"], "google");
    assert_eq!(
        connections["connections"][0]["provider_email"],
        "person@example.invalid"
    );
    assert!(connections["connections"][0].get("subject").is_none());
}

#[sqlx::test(migrations = "./migrations")]
async fn reauthentication_requires_password_and_updates_only_server_evidence(pool: PgPool) {
    let tokens = local_session(&pool, false).await;
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    assert!(matches!(
        state
            .google_service
            .as_ref()
            .unwrap()
            .create_link(session_actor(&tokens), link_request())
            .await,
        Err(GoogleAuthError::ReauthenticationRequired)
    ));
    state
        .sessions()
        .refresh(&tokens.refresh_token)
        .await
        .unwrap();
    assert!(matches!(
        state
            .google_service
            .as_ref()
            .unwrap()
            .create_link(session_actor(&tokens), link_request())
            .await,
        Err(GoogleAuthError::ReauthenticationRequired)
    ));
    sqlx::query("UPDATE auth.sessions SET last_authenticated_at = clock_timestamp() - interval '11 minutes', reauthentication_method = 'password', reauthentication_password_version = (SELECT (extract(epoch FROM password_changed_at) * 1000000)::bigint FROM auth.password_credentials WHERE account_id = $2) WHERE id = $1")
        .bind(tokens.sid)
        .bind(tokens.account_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        state
            .google_service
            .as_ref()
            .unwrap()
            .create_link(session_actor(&tokens), link_request())
            .await,
        Err(GoogleAuthError::ReauthenticationRequired)
    ));
    let app = crate::routes::router(state.clone());
    let mut wrong = json_post_request(
        "/api/v1/auth/reauthenticate",
        json!({"password":"wrong-password"}),
    );
    wrong.headers_mut().insert(
        header::AUTHORIZATION,
        format!("Bearer {}", tokens.access_token).parse().unwrap(),
    );
    assert_eq!(
        send_google_request(&app, wrong).await.0,
        StatusCode::UNAUTHORIZED
    );
    let mut correct = json_post_request(
        "/api/v1/auth/reauthenticate",
        json!({"password":"TestPass123!"}),
    );
    correct.headers_mut().insert(
        header::AUTHORIZATION,
        format!("Bearer {}", tokens.access_token).parse().unwrap(),
    );
    assert_eq!(
        send_google_request(&app, correct).await.0,
        StatusCode::NO_CONTENT
    );
    assert!(state
        .google_service
        .as_ref()
        .unwrap()
        .create_link(session_actor(&tokens), link_request())
        .await
        .is_ok());
    let evidence: (String, String, i64) = sqlx::query_as("SELECT authentication_method, reauthentication_method, (SELECT count(*) FROM auth.sessions) FROM auth.sessions WHERE id = $1")
        .bind(tokens.sid)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(evidence, ("legacy".into(), "password".into(), 1));
}

#[sqlx::test(migrations = "./migrations")]
async fn logout_credential_change_and_new_reauthentication_invalidate_pending_links(pool: PgPool) {
    let tokens = local_session(&pool, true).await;
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.as_ref().unwrap();
    let (attempt, proof) = verified_link_attempt(service, &tokens).await;
    state
        .sessions()
        .reauthenticate(session_actor(&tokens), "TestPass123!".into())
        .await
        .unwrap();
    assert!(matches!(
        service.exchange(proof, &attempt.attempt_secret).await,
        Err(GoogleAuthError::ReauthenticationRequired)
    ));
    let (attempt, proof) = verified_link_attempt(service, &tokens).await;
    sqlx::query("UPDATE auth.password_credentials SET password_changed_at = clock_timestamp() WHERE account_id = $1")
        .bind(tokens.account_id)
        .execute(&pool)
        .await
        .unwrap();
    assert!(matches!(
        service.exchange(proof, &attempt.attempt_secret).await,
        Err(GoogleAuthError::ReauthenticationRequired)
    ));
    state
        .sessions()
        .reauthenticate(session_actor(&tokens), "TestPass123!".into())
        .await
        .unwrap();
    let (attempt, proof) = verified_link_attempt(service, &tokens).await;
    state
        .sessions()
        .logout(&tokens.refresh_token)
        .await
        .unwrap();
    assert!(matches!(
        service.exchange(proof, &attempt.attempt_secret).await,
        Err(GoogleAuthError::InvalidAccess)
    ));
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM auth.oauth_identities")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
}

#[sqlx::test(migrations = "./migrations")]
async fn link_cannot_take_an_identity_or_a_contact_from_another_account(pool: PgPool) {
    let tokens = local_session(&pool, true).await;
    let other: uuid::Uuid =
        sqlx::query_scalar("INSERT INTO auth.accounts DEFAULT VALUES RETURNING id")
            .fetch_one(&pool)
            .await
            .unwrap();
    sqlx::query("INSERT INTO auth.oauth_identities(account_id, provider, provider_subject) VALUES ($1, 'google', 'verified-google-subject')")
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.as_ref().unwrap();
    let (attempt, proof) = verified_link_attempt(service, &tokens).await;
    assert!(matches!(
        service.exchange(proof, &attempt.attempt_secret).await,
        Err(GoogleAuthError::IdentityLinked)
    ));
    sqlx::query("DELETE FROM auth.oauth_identities WHERE account_id = $1")
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE auth.accounts SET email = 'person@example.invalid', email_verified_at = clock_timestamp() WHERE id = $1")
        .bind(other)
        .execute(&pool)
        .await
        .unwrap();
    let (attempt, proof) = verified_link_attempt(service, &tokens).await;
    assert!(matches!(
        service.exchange(proof, &attempt.attempt_secret).await,
        Err(GoogleAuthError::LinkConflict)
    ));
    sqlx::query("INSERT INTO auth.oauth_identities(account_id, provider, provider_subject) VALUES ($1, 'google', 'different-subject')")
        .bind(tokens.account_id)
        .execute(&pool)
        .await
        .unwrap();
    let (attempt, proof) = verified_link_attempt(service, &tokens).await;
    assert!(matches!(
        service.exchange(proof, &attempt.attempt_secret).await,
        Err(GoogleAuthError::ProviderLinked)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn notification_failure_rolls_back_link_event_and_attempt_consumption(pool: PgPool) {
    let tokens = local_session(&pool, true).await;
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.as_ref().unwrap();
    let (attempt, proof) = verified_link_attempt(service, &tokens).await;
    sqlx::raw_sql("CREATE FUNCTION auth.reject_test_notice() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'Injected outbox failure'; END; $$; CREATE TRIGGER reject_test_notice BEFORE INSERT ON auth.security_notification_outbox FOR EACH ROW EXECUTE FUNCTION auth.reject_test_notice();")
        .execute(&pool)
        .await
        .unwrap();
    assert!(service
        .exchange(proof, &attempt.attempt_secret)
        .await
        .is_err());
    let counts: (i64, i64, String) = sqlx::query_as("SELECT (SELECT count(*) FROM auth.oauth_identities), (SELECT count(*) FROM auth.account_security_events), status FROM auth.federated_auth_attempts WHERE id = $1")
        .bind(attempt.attempt_id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(counts, (0, 0, "verified".into()));
}

#[sqlx::test(migrations = "./migrations")]
async fn simultaneous_link_attempts_have_one_event_and_one_notice(pool: PgPool) {
    let tokens = local_session(&pool, true).await;
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let service = state.google_service.as_ref().unwrap();
    let (first, first_proof) = verified_link_attempt(service, &tokens).await;
    let (second, second_proof) = verified_link_attempt(service, &tokens).await;
    let (first, second) = tokio::join!(
        service.exchange(first_proof, &first.attempt_secret),
        service.exchange(second_proof, &second.attempt_secret)
    );
    assert!(first.is_ok() && second.is_ok());
    let counts: (i64, i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM auth.oauth_identities), (SELECT count(*) FROM auth.account_security_events), (SELECT count(*) FROM auth.security_notification_outbox)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(counts, (1, 1, 1));
}

#[sqlx::test(migrations = "./migrations")]
async fn link_http_requires_authoritative_access_and_returns_no_tokens(pool: PgPool) {
    let tokens = local_session(&pool, true).await;
    let (state, _) = google_test_state(&pool, EmailOwnership::ProviderVerified);
    let app = crate::routes::router(state.clone());
    let body = json!({"redirect_target":"desktop","code_challenge":URL_SAFE_NO_PAD.encode(hash(VERIFIER))});
    assert_eq!(
        send_google_request(
            &app,
            json_post_request("/api/v1/auth/google/link-attempts", body.clone())
        )
        .await
        .0,
        StatusCode::UNAUTHORIZED
    );
    let mut injected = body.clone();
    injected["account_id"] = json!(tokens.account_id);
    let mut request = json_post_request("/api/v1/auth/google/link-attempts", injected);
    request.headers_mut().insert(
        header::AUTHORIZATION,
        format!("Bearer {}", tokens.access_token).parse().unwrap(),
    );
    assert_eq!(
        send_google_request(&app, request).await.0,
        StatusCode::BAD_REQUEST
    );
    let mut request = json_post_request("/api/v1/auth/google/link-attempts", body);
    request.headers_mut().insert(
        header::AUTHORIZATION,
        format!("Bearer {}", tokens.access_token).parse().unwrap(),
    );
    let (status, _, attempt) = send_google_request(&app, request).await;
    assert_eq!(status, StatusCode::CREATED);
    let attempt = GoogleAttemptResponse {
        attempt_id: serde_json::from_value(attempt["attempt_id"].clone()).unwrap(),
        attempt_secret: attempt["attempt_secret"].as_str().unwrap().into(),
        authorization_url: attempt["authorization_url"].as_str().unwrap().into(),
        expires_at: attempt["expires_at"].as_str().unwrap().into(),
    };
    let (attempt, proof) =
        complete_browser_authorization(state.google_service.as_ref().unwrap(), attempt).await;
    let mut request = json_post_request(
        "/api/v1/auth/google/exchange",
        json!({"attempt_id":proof.attempt_id,"completion_code":proof.completion_code,"code_verifier":proof.code_verifier}),
    );
    request.headers_mut().insert(
        "x-auth-attempt-secret",
        attempt.attempt_secret.parse().unwrap(),
    );
    let (status, headers, body) = send_google_request(&app, request).await;
    assert_eq!(status, StatusCode::NO_CONTENT);
    assert_eq!(headers[header::CACHE_CONTROL], "no-store");
    assert!(body.is_null());
}

#[sqlx::test(migrations = "./migrations")]
async fn account_without_verified_contact_never_queues_notice_to_google_email(pool: PgPool) {
    use crate::services::google_auth_service::GoogleExchangeResult;
    let (state, _) = google_test_state(&pool, EmailOwnership::Unverified);
    let service = state.google_service.as_ref().unwrap();
    let (attempt, proof) = verified_login_attempt(service).await;
    let Ok(GoogleExchangeResult::Tokens(tokens)) =
        service.exchange(proof, &attempt.attempt_secret).await
    else {
        panic!("Google login failed");
    };
    let (attempt, proof) = verified_link_attempt(service, &tokens).await;
    assert!(matches!(
        service.exchange(proof, &attempt.attempt_secret).await,
        Ok(GoogleExchangeResult::Linked)
    ));
    let counts: (i64, i64) = sqlx::query_as("SELECT (SELECT count(*) FROM auth.account_security_events), (SELECT count(*) FROM auth.security_notification_outbox)")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(counts, (1, 0));
}

#[tokio::test]
async fn reauthentication_budget_is_account_scoped_and_shared_across_instances() {
    use crate::protection::{auth_rate_limiter::AuthRateLimiter, rate_limit::RateLimitDecision};
    let redis = std::env::var("REDIS_URL").expect("Redis test requires REDIS_URL");
    let first = AuthRateLimiter::for_tests(redis.clone());
    let second = AuthRateLimiter::for_tests(redis);
    let account = uuid::Uuid::new_v4();
    for index in 0..6 {
        let limiter = if index % 2 == 0 { &first } else { &second };
        assert!(matches!(
            limiter.check_reauthentication(account).await.unwrap(),
            RateLimitDecision::Allowed
        ));
    }
    assert!(matches!(
        second.check_reauthentication(account).await.unwrap(),
        RateLimitDecision::Limited { .. }
    ));
    assert!(matches!(
        second
            .check_reauthentication(uuid::Uuid::new_v4())
            .await
            .unwrap(),
        RateLimitDecision::Allowed
    ));
    let unavailable = AuthRateLimiter::for_tests("redis://127.0.0.1:1/0".into());
    assert!(unavailable.check_reauthentication(account).await.is_err());
}
