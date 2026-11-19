use std::{collections::HashMap, sync::Arc};

use auth_service::{
    federated_identity::{
        AuthorizationCode, AuthorizationRequest, EmailOwnership, FederatedIdentityProvider,
        VerifiedIdentity,
    },
    google_oidc::GoogleOidcProvider,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};
use reqwest::Url;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;
use tokio::sync::Semaphore;
use uuid::Uuid;

use super::{
    google_auth_config::{GoogleAuthConfig, GoogleReturnTarget},
    session_service::{SessionError, SessionService},
};
use crate::{
    dto::{
        google::{
            CreateGoogleAttempt, ExchangeGoogleAttempt, GoogleAttemptResponse, GoogleConnection,
            GoogleConnections, LinkGoogleAttempt,
        },
        sessions::TokenResponse,
    },
    repositories::{
        federated_auth_repository::{
            CompletedGoogleLogin, FederatedAttempt, FederatedAuthRepository, FederatedProof,
            IdentityResolution, NewFederatedAttempt,
        },
        google_connections_repository::{GoogleConnectionsRepository, LinkAdmission, LinkResult},
        session_authentication_repository::SessionActor,
        sessions_repository::SessionsRepository,
    },
    validators::auth_validator::is_valid_email_format,
};

const MAX_CONCURRENT_CALLBACKS: usize = 16;

pub enum GoogleAuthError {
    Malformed,
    InvalidProof,
    InvalidAccess,
    ReauthenticationRequired,
    IdentityLinked,
    ProviderLinked,
    LinkConflict,
    AccountBlocked,
    LinkRequired,
    Unavailable,
    CommitUncertain,
    Database(sqlx::Error),
    Session(SessionError),
}

impl From<sqlx::Error> for GoogleAuthError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

#[derive(Clone)]
pub struct GoogleAuthService {
    repository: FederatedAuthRepository,
    sessions: SessionService,
    provider: Arc<dyn FederatedIdentityProvider>,
    authorization_url: Url,
    targets: Arc<HashMap<String, GoogleReturnTarget>>,
    callback_slots: Arc<Semaphore>,
}

pub enum GoogleExchangeResult {
    Tokens(TokenResponse),
    Linked,
}

pub struct BrowserAuthorization {
    pub url: String,
    pub state: String,
}

pub struct CallbackResult {
    pub redirect: Url,
}

pub struct GoogleCallback<'a> {
    pub state: &'a str,
    pub cookie_state: &'a str,
    pub code: Option<&'a str>,
    pub denied: bool,
}

impl GoogleAuthService {
    pub fn new(
        repository: FederatedAuthRepository,
        sessions: SessionService,
        config: GoogleAuthConfig,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            repository,
            sessions,
            provider: Arc::new(GoogleOidcProvider::new(config.provider)?),
            authorization_url: config.authorization_url,
            targets: Arc::new(config.targets),
            callback_slots: Arc::new(Semaphore::new(MAX_CONCURRENT_CALLBACKS)),
        })
    }

    #[cfg(test)]
    pub(crate) fn with_provider(mut self, provider: Arc<dyn FederatedIdentityProvider>) -> Self {
        self.provider = provider;
        self
    }

    pub async fn create(
        &self,
        request: CreateGoogleAttempt,
    ) -> Result<GoogleAttemptResponse, GoogleAuthError> {
        self.create_attempt(request, None).await
    }

    pub async fn create_link(
        &self,
        actor: SessionActor,
        request: LinkGoogleAttempt,
    ) -> Result<GoogleAttemptResponse, GoogleAuthError> {
        let target = self
            .targets
            .get(&request.redirect_target)
            .ok_or(GoogleAuthError::Malformed)?;
        self.create_attempt(
            CreateGoogleAttempt {
                client_type: target.client_type,
                device_name: None,
                redirect_target: request.redirect_target,
                code_challenge: request.code_challenge,
            },
            Some(actor),
        )
        .await
    }

    async fn create_attempt(
        &self,
        request: CreateGoogleAttempt,
        actor: Option<SessionActor>,
    ) -> Result<GoogleAttemptResponse, GoogleAuthError> {
        let target = self
            .targets
            .get(&request.redirect_target)
            .ok_or(GoogleAuthError::Malformed)?;
        if target.client_type.as_str() != request.client_type.as_str()
            || request.device_name.as_ref().is_some_and(|name| {
                name.chars().count() > 120 || name.chars().any(char::is_control)
            })
        {
            return Err(GoogleAuthError::Malformed);
        }

        let challenge = decode_secret(&request.code_challenge).ok_or(GoogleAuthError::Malformed)?;
        let id = Uuid::new_v4();
        let secret = random_secret()?;
        let ticket = random_secret()?;
        let state = derive(&ticket, b"google-state");
        let nonce = derive(&ticket, b"google-nonce");
        let prepared = NewFederatedAttempt {
            id,
            client_type: request.client_type.as_str(),
            device_name: request.device_name.as_deref(),
            redirect_target: &request.redirect_target,
            secret_hash: &hash(&secret),
            ticket_hash: &hash(&ticket),
            state_hash: &hash(&state),
            nonce_hash: &hash(&nonce),
            challenge: &challenge,
        };

        let expires_at = match actor {
            None => self.repository.create(prepared).await?,
            Some(actor) => self.create_link_attempt(actor, prepared).await?,
        };

        let mut url = self.authorization_url.clone();
        url.query_pairs_mut().append_pair("ticket", &ticket);
        Ok(GoogleAttemptResponse {
            attempt_id: id,
            attempt_secret: secret,
            authorization_url: url.into(),
            expires_at,
        })
    }

    async fn create_link_attempt(
        &self,
        actor: SessionActor,
        attempt: NewFederatedAttempt<'_>,
    ) -> Result<String, GoogleAuthError> {
        match GoogleConnectionsRepository::create(&self.repository.pool, actor, attempt).await? {
            LinkAdmission::Created(expires_at) => Ok(expires_at),
            LinkAdmission::InvalidSession => Err(GoogleAuthError::InvalidAccess),
            LinkAdmission::StaleEvidence => Err(GoogleAuthError::ReauthenticationRequired),
            LinkAdmission::InvalidTarget => Err(GoogleAuthError::Malformed),
        }
    }

    pub async fn authorize(&self, ticket: &str) -> Result<BrowserAuthorization, GoogleAuthError> {
        decode_secret(ticket).ok_or(GoogleAuthError::InvalidProof)?;
        let state = derive(ticket, b"google-state");
        let nonce = derive(ticket, b"google-nonce");
        let url = self
            .provider
            .authorization_url(AuthorizationRequest {
                state: &state,
                nonce: &nonce,
            })
            .map_err(|_| GoogleAuthError::Unavailable)?;
        if self.repository.authorize(&hash(ticket)).await?.is_none() {
            return Err(GoogleAuthError::InvalidProof);
        }
        Ok(BrowserAuthorization { url, state })
    }

    pub async fn callback(
        &self,
        request: GoogleCallback<'_>,
    ) -> Result<CallbackResult, GoogleAuthError> {
        decode_secret(request.state).ok_or(GoogleAuthError::InvalidProof)?;
        if !bool::from(
            request
                .state
                .as_bytes()
                .ct_eq(request.cookie_state.as_bytes()),
        ) {
            return Err(GoogleAuthError::InvalidProof);
        }

        let _slot = self
            .callback_slots
            .try_acquire()
            .map_err(|_| GoogleAuthError::Unavailable)?;
        let id = self
            .repository
            .claim_callback(&hash(request.state))
            .await?
            .ok_or(GoogleAuthError::InvalidProof)?;
        let attempt = self.repository.find(id).await?;
        let mut redirect = self
            .targets
            .get(&attempt.redirect_target)
            .ok_or(GoogleAuthError::Unavailable)?
            .url
            .clone();
        redirect
            .query_pairs_mut()
            .append_pair("attempt_id", &id.to_string());
        let Some(code) = request.code else {
            return self.failed_callback(id, request.denied, redirect).await;
        };

        let nonce_hash: [u8; 32] = attempt
            .oidc_nonce_hash
            .try_into()
            .map_err(|_| GoogleAuthError::Unavailable)?;
        let identity = match self
            .provider
            .exchange_code(AuthorizationCode {
                code,
                expected_nonce_hash: &nonce_hash,
            })
            .await
        {
            Ok(identity) => identity,
            Err(error) => {
                tracing::warn!(
                    event = "google_identity_verification_failed",
                    reason = ?error,
                    "Google identity verification did not complete"
                );
                return self.failed_callback(id, false, redirect).await;
            }
        };
        if identity.issuer != "https://accounts.google.com"
            || identity.subject.is_empty()
            || identity
                .email
                .as_ref()
                .is_some_and(|email| !is_valid_email_format(&email.address.to_lowercase()))
        {
            self.repository.fail(id, false).await?;
            return Err(GoogleAuthError::InvalidProof);
        }

        let completion = self.record_identity_proof(id, &identity).await?;
        redirect
            .query_pairs_mut()
            .append_pair("completion_code", &completion);
        Ok(CallbackResult { redirect })
    }

    async fn failed_callback(
        &self,
        attempt_id: Uuid,
        cancelled: bool,
        mut redirect: Url,
    ) -> Result<CallbackResult, GoogleAuthError> {
        self.repository.fail(attempt_id, cancelled).await?;
        let error = if cancelled { "cancelled" } else { "failed" };
        redirect.query_pairs_mut().append_pair("error", error);
        Ok(CallbackResult { redirect })
    }

    async fn record_identity_proof(
        &self,
        attempt_id: Uuid,
        identity: &VerifiedIdentity,
    ) -> Result<String, GoogleAuthError> {
        let completion = random_secret()?;
        let valid_until = i64::try_from(identity.expires_at)
            .ok()
            .and_then(|seconds| sqlx::types::chrono::DateTime::from_timestamp(seconds, 0))
            .ok_or(GoogleAuthError::InvalidProof)?;
        let email = identity
            .email
            .as_ref()
            .map(|email| email.address.to_lowercase());
        if !self
            .repository
            .record_proof(
                attempt_id,
                FederatedProof {
                    subject: &identity.subject,
                    email: email.as_deref(),
                    email_authoritative: identity
                        .email
                        .as_ref()
                        .is_some_and(|email| email.ownership == EmailOwnership::ProviderVerified),
                    valid_until,
                    completion_hash: &hash(&completion),
                },
            )
            .await?
        {
            return Err(GoogleAuthError::InvalidProof);
        }
        Ok(completion)
    }

    pub async fn exchange(
        &self,
        request: ExchangeGoogleAttempt,
        secret: &str,
    ) -> Result<GoogleExchangeResult, GoogleAuthError> {
        decode_secret(secret).ok_or(GoogleAuthError::InvalidProof)?;
        decode_secret(&request.completion_code).ok_or(GoogleAuthError::InvalidProof)?;
        if !(43..=128).contains(&request.code_verifier.len())
            || !request
                .code_verifier
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"-._~".contains(&byte))
        {
            return Err(GoogleAuthError::InvalidProof);
        }

        let attempt =
            self.repository
                .find(request.attempt_id)
                .await
                .map_err(|error| match error {
                    sqlx::Error::RowNotFound => GoogleAuthError::InvalidProof,
                    other => GoogleAuthError::Database(other),
                })?;
        validate_exchange(&attempt, &request, secret)?;
        if attempt.purpose == "link" {
            return self.finish_link(&attempt).await;
        }
        self.finish_login(&attempt).await
    }

    async fn finish_login(
        &self,
        attempt: &FederatedAttempt,
    ) -> Result<GoogleExchangeResult, GoogleAuthError> {
        let mut tx = self.repository.pool.begin().await?;
        let (account_id, identity_id) =
            match FederatedAuthRepository::resolve_identity(&mut tx, attempt).await? {
                IdentityResolution::Allowed {
                    account_id,
                    identity_id,
                } => (account_id, identity_id),
                IdentityResolution::ContactConflict => return Err(GoogleAuthError::LinkRequired),
                IdentityResolution::AccountBlocked => return Err(GoogleAuthError::AccountBlocked),
            };

        let session_id =
            FederatedAuthRepository::open_session(&mut tx, attempt, account_id, identity_id)
                .await?;
        let session = SessionsRepository::lock(&mut tx, session_id).await?;
        if !FederatedAuthRepository::consume(
            &mut tx,
            attempt.id,
            CompletedGoogleLogin {
                account_id,
                identity_id,
                session_id,
            },
        )
        .await?
        {
            return Err(GoogleAuthError::InvalidProof);
        }

        let tokens = self
            .sessions
            .issue_tokens(&mut tx, &session)
            .await
            .map_err(GoogleAuthError::Session)?;
        tx.commit()
            .await
            .map_err(|_| GoogleAuthError::CommitUncertain)?;
        Ok(GoogleExchangeResult::Tokens(tokens))
    }

    async fn finish_link(
        &self,
        attempt: &FederatedAttempt,
    ) -> Result<GoogleExchangeResult, GoogleAuthError> {
        let mut tx = self.repository.pool.begin().await?;
        match GoogleConnectionsRepository::link(&mut tx, attempt).await? {
            LinkResult::Linked => {}
            LinkResult::InvalidSession => return Err(GoogleAuthError::InvalidAccess),
            LinkResult::StaleEvidence => return Err(GoogleAuthError::ReauthenticationRequired),
            LinkResult::InvalidAttempt => return Err(GoogleAuthError::InvalidProof),
            LinkResult::IdentityConflict => return Err(GoogleAuthError::IdentityLinked),
            LinkResult::ProviderConflict => return Err(GoogleAuthError::ProviderLinked),
            LinkResult::ContactConflict => return Err(GoogleAuthError::LinkConflict),
        }
        tx.commit()
            .await
            .map_err(|_| GoogleAuthError::CommitUncertain)?;
        Ok(GoogleExchangeResult::Linked)
    }

    pub async fn connections(
        &self,
        actor: SessionActor,
    ) -> Result<GoogleConnections, GoogleAuthError> {
        let rows = GoogleConnectionsRepository::list(&self.repository.pool, actor).await?;
        if rows.is_empty() {
            return Err(GoogleAuthError::InvalidAccess);
        }
        Ok(GoogleConnections {
            connections: rows
                .into_iter()
                .filter_map(|row| {
                    Some(GoogleConnection {
                        provider: row.provider?,
                        provider_email: row.provider_email,
                        linked_at: row.linked_at?.to_rfc3339(),
                    })
                })
                .collect(),
        })
    }

    pub async fn cleanup(&self) -> Result<(), GoogleAuthError> {
        self.repository.cleanup().await.map_err(Into::into)
    }
}

fn validate_exchange(
    attempt: &FederatedAttempt,
    request: &ExchangeGoogleAttempt,
    secret: &str,
) -> Result<(), GoogleAuthError> {
    let completion = attempt
        .completion_code_hash
        .as_deref()
        .ok_or(GoogleAuthError::InvalidProof)?;
    if !attempt.usable
        || !bool::from(
            hash(secret)
                .as_slice()
                .ct_eq(&attempt.initiator_secret_hash),
        )
        || !bool::from(hash(&request.completion_code).as_slice().ct_eq(completion))
        || !bool::from(
            hash(&request.code_verifier)
                .as_slice()
                .ct_eq(&attempt.code_challenge),
        )
    {
        return Err(GoogleAuthError::InvalidProof);
    }
    Ok(())
}

pub(crate) fn hash(value: &str) -> [u8; 32] {
    Sha256::digest(value.as_bytes()).into()
}

fn random_secret() -> Result<String, GoogleAuthError> {
    let mut bytes = zeroize::Zeroizing::new([0u8; 32]);
    OsRng
        .try_fill_bytes(bytes.as_mut())
        .map_err(|_| GoogleAuthError::Unavailable)?;
    Ok(URL_SAFE_NO_PAD.encode(bytes.as_ref()))
}

fn derive(ticket: &str, purpose: &[u8]) -> String {
    let mut digest = Sha256::new();
    digest.update(purpose);
    digest.update([0]);
    digest.update(ticket.as_bytes());
    URL_SAFE_NO_PAD.encode(digest.finalize())
}

fn decode_secret(value: &str) -> Option<[u8; 32]> {
    if value.len() != 43 {
        return None;
    }
    URL_SAFE_NO_PAD.decode(value).ok()?.try_into().ok()
}
