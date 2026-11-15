use std::{
    collections::HashMap,
    io::Write,
    process::{Command, Stdio},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex as StdMutex, OnceLock,
    },
};

use axum::{
    extract::{Form, State},
    http::StatusCode,
    routing::{get, post},
    Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::task::JoinSet;

use super::*;
use crate::federated_identity::EmailOwnership;

const CLIENT_ID: &str = "test-client.apps.googleusercontent.com";
const CALLBACK: &str = "https://auth.example.invalid/api/v1/auth/google/callback";
const NONCE: &str = "test-nonce-0123456789abcdefghijklmnopqrstuvwxyz";

fn config() -> GoogleOidcConfig {
    GoogleOidcConfig::new(CLIENT_ID.into(), "test-only-client-secret".into(), CALLBACK).unwrap()
}

fn run_openssl(arguments: &[&str], input: Option<&[u8]>) -> Vec<u8> {
    let mut command = Command::new("openssl");
    command
        .args(arguments)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    if input.is_some() {
        command.stdin(Stdio::piped());
    } else {
        command.stdin(Stdio::null());
    }
    let mut child = command.spawn().expect("OpenSSL is required for OIDC tests");
    if let Some(input) = input {
        child.stdin.take().unwrap().write_all(input).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "OpenSSL test key generation failed"
    );
    output.stdout
}

fn signing_material() -> &'static (Vec<u8>, Value) {
    static MATERIAL: OnceLock<(Vec<u8>, Value)> = OnceLock::new();
    MATERIAL.get_or_init(|| {
        let private_key = run_openssl(
            &[
                "genpkey",
                "-algorithm",
                "RSA",
                "-pkeyopt",
                "rsa_keygen_bits:2048",
            ],
            None,
        );
        let modulus_output = run_openssl(&["rsa", "-modulus", "-noout"], Some(&private_key));
        let modulus_text = String::from_utf8(modulus_output).unwrap();
        let modulus_hex = modulus_text.trim().strip_prefix("Modulus=").unwrap();
        let modulus_bytes: Vec<u8> = (0..modulus_hex.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&modulus_hex[index..index + 2], 16).unwrap())
            .collect();
        let key_set = json!({
            "keys": [{
                "kid": "test-key",
                "kty": "RSA",
                "alg": "RS256",
                "use": "sig",
                "n": URL_SAFE_NO_PAD.encode(modulus_bytes),
                "e": "AQAB"
            }]
        });
        let private_key_der = run_openssl(
            &["rsa", "-traditional", "-outform", "DER"],
            Some(&private_key),
        );
        (private_key_der, key_set)
    })
}

fn claims() -> Value {
    let now = unix_time().unwrap();
    json!({"iss":ISSUER, "sub":"google-subject", "aud":CLIENT_ID, "iat":now - 10, "exp":now + 3600,
        "nonce":NONCE, "email":"person@gmail.com", "email_verified":true})
}

fn sign_identity_token(claims: &Value, kid: &str) -> String {
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some(kid.into());
    encode(
        &header,
        claims,
        &EncodingKey::from_rsa_der(&signing_material().0),
    )
    .unwrap()
}

struct MockState {
    token_response: StdMutex<(StatusCode, String)>,
    key_response: StdMutex<(StatusCode, String)>,
    token_requests: AtomicUsize,
    key_requests: AtomicUsize,
    form: StdMutex<HashMap<String, String>>,
}

struct TestProvider {
    provider: Arc<GoogleOidcProvider>,
    state: Arc<MockState>,
    task: tokio::task::JoinHandle<()>,
}

impl Drop for TestProvider {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl TestProvider {
    async fn new(token: &str) -> Self {
        let state = Arc::new(MockState {
            token_response: StdMutex::new((
                StatusCode::OK,
                json!({"id_token":token, "access_token":"discarded-test-secret"}).to_string(),
            )),
            key_response: StdMutex::new((StatusCode::OK, signing_material().1.to_string())),
            token_requests: AtomicUsize::new(0),
            key_requests: AtomicUsize::new(0),
            form: StdMutex::new(HashMap::new()),
        });
        let router = Router::new()
            .route(
                "/token",
                post(
                    |State(state): State<Arc<MockState>>,
                     Form(form): Form<HashMap<String, String>>| async move {
                        state.token_requests.fetch_add(1, Ordering::SeqCst);
                        *state.form.lock().unwrap() = form;
                        state.token_response.lock().unwrap().clone()
                    },
                ),
            )
            .route(
                "/keys",
                get(|State(state): State<Arc<MockState>>| async move {
                    state.key_requests.fetch_add(1, Ordering::SeqCst);
                    let (status, body) = state.key_response.lock().unwrap().clone();
                    (status, [("cache-control", "public, max-age=3600")], body)
                }),
            )
            .with_state(state.clone());
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let task = tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        });
        let mut provider = GoogleOidcProvider::new(config()).unwrap();
        provider.client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap();
        provider.token_endpoint = Url::parse(&format!("http://{address}/token")).unwrap();
        provider.jwks_endpoint = Url::parse(&format!("http://{address}/keys")).unwrap();
        Self {
            provider: Arc::new(provider),
            state,
            task,
        }
    }

    async fn exchange(&self) -> Result<VerifiedIdentity, FederatedIdentityError> {
        self.provider
            .exchange_code(AuthorizationCode {
                code: "test-authorization-code",
                expected_nonce_hash: &Sha256::digest(NONCE).into(),
            })
            .await
    }

    fn set_identity_token(&self, token: &str) {
        *self.state.token_response.lock().unwrap() =
            (StatusCode::OK, json!({"id_token":token}).to_string());
    }
}

#[test]
fn configuration_and_authorization_requests_are_strict_and_do_not_expose_client_secret() {
    for callback in [
        "http://auth.example.invalid/api/v1/auth/google/callback",
        "https://user:password@auth.example.invalid/api/v1/auth/google/callback",
        "https://auth.example.invalid/callback",
        "https://auth.example.invalid/api/v1/auth/google/callback?next=elsewhere",
        "https://auth.example.invalid/api/v1/auth/google/callback#fragment",
    ] {
        assert!(GoogleOidcConfig::new(CLIENT_ID.into(), "test-secret".into(), callback).is_err());
    }
    for (client, secret) in [
        ("", "secret"),
        ("arbitrary-client", "secret"),
        (CLIENT_ID, ""),
        (CLIENT_ID, " secret "),
    ] {
        let error = GoogleOidcConfig::new(client.into(), secret.into(), CALLBACK)
            .err()
            .unwrap();
        assert_eq!(error, FederatedIdentityError::Configuration);
        assert_eq!(format!("{error:?}"), "Configuration");
    }
    let provider = GoogleOidcProvider::new(config()).unwrap();
    let url = provider
        .authorization_url(AuthorizationRequest {
            state: NONCE,
            nonce: NONCE,
        })
        .unwrap();
    let url = Url::parse(&url).unwrap();
    assert_eq!(url.origin().ascii_serialization(), ISSUER);
    let query: HashMap<_, _> = url.query_pairs().into_owned().collect();
    assert_eq!(query["scope"], "openid email");
    assert_eq!(query["response_type"], "code");
    assert_eq!(query["redirect_uri"], CALLBACK);
    assert_eq!(query["state"], NONCE);
    assert_eq!(query["nonce"], NONCE);
    assert!(!query.contains_key("client_secret"));
    assert!(!query.contains_key("access_type"));
    for value in [
        "",
        "short",
        "invalid value with whitespace................................",
    ] {
        assert!(provider
            .authorization_url(AuthorizationRequest {
                state: value,
                nonce: NONCE
            })
            .is_err());
        assert!(provider
            .authorization_url(AuthorizationRequest {
                state: NONCE,
                nonce: value
            })
            .is_err());
    }
}

#[tokio::test]
async fn signed_identity_and_email_authority_follow_google_rules() {
    let fixture = TestProvider::new(&sign_identity_token(&claims(), "test-key")).await;
    for (address, verified, domain, expected) in [
        (
            Some("person@gmail.com"),
            true,
            None,
            Some(EmailOwnership::ProviderVerified),
        ),
        (
            Some("person@gmail.com"),
            false,
            None,
            Some(EmailOwnership::Unverified),
        ),
        (
            Some("person@workspace.invalid"),
            true,
            Some("workspace.invalid"),
            Some(EmailOwnership::ProviderVerified),
        ),
        (
            Some("person@external.invalid"),
            true,
            None,
            Some(EmailOwnership::Unverified),
        ),
        (
            Some("person@external.invalid"),
            false,
            Some("workspace.invalid"),
            Some(EmailOwnership::Unverified),
        ),
        (None, false, None, None),
    ] {
        let mut payload = claims();
        payload["email"] = json!(address);
        payload["email_verified"] = json!(verified);
        payload["hd"] = json!(domain);
        fixture.set_identity_token(&sign_identity_token(&payload, "test-key"));
        let identity = fixture.exchange().await.unwrap();
        assert_eq!(identity.subject, "google-subject");
        assert_eq!(identity.issuer, ISSUER);
        assert_eq!(
            identity.email.as_ref().map(|email| email.ownership),
            expected
        );
        assert_eq!(
            identity.email.as_ref().map(|email| email.address.as_str()),
            address
        );
    }
    let form = fixture.state.form.lock().unwrap();
    assert_eq!(form["grant_type"], "authorization_code");
    assert_eq!(form["client_id"], CLIENT_ID);
    assert_eq!(form["client_secret"], "test-only-client-secret");
    assert_eq!(form["redirect_uri"], CALLBACK);
    assert_eq!(fixture.state.key_requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn invalid_claims_and_missing_required_claims_are_rejected() {
    let fixture = TestProvider::new(&sign_identity_token(&claims(), "test-key")).await;
    let now = unix_time().unwrap();
    for (name, value) in [
        ("iss", json!("https://attacker.invalid")),
        ("aud", json!("another-client")),
        ("azp", json!("another-client")),
        ("exp", json!(now - 1)),
        ("iat", json!(now + 300)),
        ("sub", json!("")),
        ("sub", json!("x".repeat(256))),
        (
            "nonce",
            json!("other-nonce-0123456789abcdefghijklmnopqrstuvwxyz"),
        ),
        ("email", json!("person@@gmail.com")),
        ("email", json!(" person@gmail.com")),
        ("email_verified", json!("true")),
        ("aud", json!([CLIENT_ID, "another-client"])),
    ] {
        let mut payload = claims();
        payload[name] = value;
        fixture.set_identity_token(&sign_identity_token(&payload, "test-key"));
        assert!(
            matches!(
                fixture.exchange().await,
                Err(FederatedIdentityError::InvalidIdentity)
            ),
            "accepted invalid {name}"
        );
    }
    for name in ["iss", "sub", "aud", "iat", "exp", "nonce"] {
        let mut payload = claims();
        payload.as_object_mut().unwrap().remove(name);
        fixture.set_identity_token(&sign_identity_token(&payload, "test-key"));
        assert!(
            matches!(
                fixture.exchange().await,
                Err(FederatedIdentityError::InvalidIdentity)
            ),
            "accepted missing {name}"
        );
    }
    let mut payload = claims();
    payload["iss"] = json!("accounts.google.com");
    payload["aud"] = json!([CLIENT_ID, "another-client"]);
    payload["azp"] = json!(CLIENT_ID);
    fixture.set_identity_token(&sign_identity_token(&payload, "test-key"));
    assert_eq!(fixture.exchange().await.unwrap().issuer, ISSUER);
}

#[tokio::test]
async fn signature_algorithm_and_untrusted_header_keys_cannot_replace_google_keys() {
    let fixture = TestProvider::new(&sign_identity_token(&claims(), "test-key")).await;
    let mut header = Header::new(Algorithm::HS256);
    header.kid = Some("test-key".into());
    fixture.set_identity_token(
        &encode(
            &header,
            &claims(),
            &EncodingKey::from_secret(b"attacker-controlled-key"),
        )
        .unwrap(),
    );
    assert!(matches!(
        fixture.exchange().await,
        Err(FederatedIdentityError::InvalidIdentity)
    ));
    assert_eq!(fixture.state.key_requests.load(Ordering::SeqCst), 0);
    let mut header = Header::new(Algorithm::RS256);
    header.kid = Some("test-key".into());
    header.jku = Some("https://attacker.invalid/keys".into());
    fixture.set_identity_token(
        &encode(
            &header,
            &claims(),
            &EncodingKey::from_rsa_der(&signing_material().0),
        )
        .unwrap(),
    );
    assert!(matches!(
        fixture.exchange().await,
        Err(FederatedIdentityError::InvalidIdentity)
    ));
    assert_eq!(fixture.state.key_requests.load(Ordering::SeqCst), 0);
    let token = sign_identity_token(&claims(), "test-key");
    let mut segments: Vec<String> = token.split('.').map(str::to_owned).collect();
    let mut signature = URL_SAFE_NO_PAD.decode(&segments[2]).unwrap();
    signature[0] ^= 1;
    segments[2] = URL_SAFE_NO_PAD.encode(signature);
    fixture.set_identity_token(&segments.join("."));
    assert!(matches!(
        fixture.exchange().await,
        Err(FederatedIdentityError::InvalidIdentity)
    ));
}

#[tokio::test]
async fn concurrent_exchanges_share_keys_and_unknown_kids_cannot_amplify_refreshes() {
    let fixture = TestProvider::new(&sign_identity_token(&claims(), "test-key")).await;
    let mut tasks = JoinSet::new();
    for _ in 0..12 {
        let provider = fixture.provider.clone();
        tasks.spawn(async move {
            provider
                .exchange_code(AuthorizationCode {
                    code: "test-code",
                    expected_nonce_hash: &Sha256::digest(NONCE).into(),
                })
                .await
                .is_ok()
        });
    }
    while let Some(result) = tasks.join_next().await {
        assert!(result.unwrap());
    }
    assert_eq!(fixture.state.key_requests.load(Ordering::SeqCst), 1);
    for index in 0..10 {
        fixture.set_identity_token(&sign_identity_token(&claims(), &format!("unknown-{index}")));
        assert!(matches!(
            fixture.exchange().await,
            Err(FederatedIdentityError::InvalidIdentity)
        ));
    }
    assert_eq!(fixture.state.key_requests.load(Ordering::SeqCst), 1);
}

#[tokio::test]
async fn provider_errors_and_oversized_or_malformed_responses_fail_without_sensitive_diagnostics() {
    let fixture = TestProvider::new(&sign_identity_token(&claims(), "test-key")).await;
    for code in [String::new(), "x".repeat(4097), "code\nwith-control".into()] {
        let error = fixture
            .provider
            .exchange_code(AuthorizationCode {
                code: &code,
                expected_nonce_hash: &Sha256::digest(NONCE).into(),
            })
            .await
            .err()
            .unwrap();
        assert_eq!(error, FederatedIdentityError::InvalidRequest);
    }
    assert_eq!(fixture.state.token_requests.load(Ordering::SeqCst), 0);
    for (status, response, expected) in [
        (
            StatusCode::TOO_MANY_REQUESTS,
            "sensitive-provider-error".into(),
            FederatedIdentityError::Unavailable,
        ),
        (
            StatusCode::BAD_REQUEST,
            "sensitive-provider-error".into(),
            FederatedIdentityError::AuthorizationRejected,
        ),
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "sensitive-provider-error".into(),
            FederatedIdentityError::Unavailable,
        ),
        (
            StatusCode::FOUND,
            "sensitive-provider-error".into(),
            FederatedIdentityError::Unavailable,
        ),
        (
            StatusCode::OK,
            "not-json".into(),
            FederatedIdentityError::InvalidIdentity,
        ),
        (
            StatusCode::OK,
            json!({"access_token":"sensitive-provider-error"}).to_string(),
            FederatedIdentityError::InvalidIdentity,
        ),
        (
            StatusCode::OK,
            "x".repeat(MAX_TOKEN_RESPONSE + 1),
            FederatedIdentityError::Unavailable,
        ),
    ] {
        *fixture.state.token_response.lock().unwrap() = (status, response);
        let error = fixture.exchange().await.err().unwrap();
        assert_eq!(error, expected);
        assert!(!format!("{error:?} {error}").contains("sensitive-provider-error"));
    }
    assert_eq!(fixture.state.key_requests.load(Ordering::SeqCst), 0);
}

#[tokio::test]
async fn invalid_jwks_fail_closed_and_do_not_trigger_repeated_downloads() {
    for body in [
        json!({"keys":[]}),
        json!({"keys":[signing_material().1["keys"][0].clone(),signing_material().1["keys"][0].clone()]}),
        json!({"keys":[{"kid":"test-key", "kty":"RSA", "n":"AA", "e":"AQAB"}]}),
    ] {
        let fixture = TestProvider::new(&sign_identity_token(&claims(), "test-key")).await;
        *fixture.state.key_response.lock().unwrap() = (StatusCode::OK, body.to_string());
        for _ in 0..2 {
            assert!(matches!(
                fixture.exchange().await,
                Err(FederatedIdentityError::Unavailable)
            ));
        }
        assert_eq!(fixture.state.key_requests.load(Ordering::SeqCst), 1);
    }
}
