use auth_service::tokens::{claims::SessionContext, issuer::AccessTokenIssuer};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::{
    dto::sessions::{LoginRequest, TokenResponse},
    repositories::{
        refresh_tokens_repository::RefreshTokensRepository as Refresh,
        sessions_repository::{
            PasswordSessionContext, PasswordSnapshot, SessionRow, SessionsRepository, Tx,
        },
    },
    services::password_processor::{PasswordProcessingError, PasswordProcessor},
    validators::auth_validator::is_valid_email_format,
};

#[derive(Debug)]
pub enum SessionError {
    Malformed,
    InvalidAccess,
    InvalidCredentials,
    InvalidRefresh,
    Busy,
    Signing,
    Database(sqlx::Error),
    CommitUncertain,
}

impl From<sqlx::Error> for SessionError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

pub trait AccessSigner: Send + Sync {
    fn sign(&self, context: &SessionContext) -> Result<(String, u64), SessionError>;
}

impl AccessSigner for AccessTokenIssuer {
    fn sign(&self, context: &SessionContext) -> Result<(String, u64), SessionError> {
        self.issue_with_lifetime(context)
            .map_err(|_| SessionError::Signing)
    }
}

#[derive(Clone)]
pub struct SessionService {
    repository: SessionsRepository,
    passwords: PasswordProcessor,
    signer: Arc<dyn AccessSigner>,
}

impl SessionService {
    pub fn new(
        repository: SessionsRepository,
        passwords: PasswordProcessor,
        signer: Arc<dyn AccessSigner>,
    ) -> Self {
        Self {
            repository,
            passwords,
            signer,
        }
    }

    pub async fn login(&self, request: LoginRequest) -> Result<TokenResponse, SessionError> {
        let email = request.email.trim().to_lowercase();

        validate_login(&request, &email)?;
        let password_snapshot = self.authenticate(&email, request.password).await?;

        let authenticated_at = sqlx::types::chrono::Utc::now();
        let mut tx = self.repository.pool.begin().await?;

        if SessionsRepository::lock_password_account(&mut tx, password_snapshot.account_id).await?
            != Some(true)
            || !SessionsRepository::password_unchanged(&mut tx, &password_snapshot).await?
        {
            return Err(SessionError::InvalidCredentials);
        }

        let session = SessionsRepository::open_password(
            &mut tx,
            PasswordSessionContext {
                credentials: &password_snapshot,
                client_type: request.client_type.as_str(),
                device_name: request.device_name.as_deref(),
                authenticated_at,
            },
        )
        .await?;

        let response = self.issue_tokens(&mut tx, &session).await?;

        tx.commit()
            .await
            .map_err(|_| SessionError::CommitUncertain)?;
        Ok(response)
    }

    pub async fn reauthenticate(
        &self,
        actor: crate::repositories::session_authentication_repository::SessionActor,
        password: String,
    ) -> Result<(), SessionError> {
        use crate::repositories::session_authentication_repository::SessionAuthenticationRepository;
        if password.chars().count() > 128 {
            return Err(SessionError::Malformed);
        }
        let snapshot =
            SessionAuthenticationRepository::password(&self.repository.pool, actor).await?;
        let matches = self
            .passwords
            .verify(
                password,
                snapshot.as_ref().map(|proof| proof.password_hash.clone()),
            )
            .await
            .map_err(|error| match error {
                PasswordProcessingError::Busy => SessionError::Busy,
                _ => SessionError::Signing,
            })?;
        if !matches {
            return Err(SessionError::InvalidCredentials);
        }
        let proof = snapshot.ok_or(SessionError::InvalidCredentials)?;
        let authenticated_at = sqlx::types::chrono::Utc::now();
        let mut tx = self.repository.pool.begin().await?;
        if SessionsRepository::lock_password_account(&mut tx, actor.account_id).await? != Some(true)
            || SessionAuthenticationRepository::lock(&mut tx, actor)
                .await?
                .is_none()
        {
            return Err(SessionError::InvalidAccess);
        }
        if !SessionsRepository::password_unchanged(&mut tx, &proof).await? {
            return Err(SessionError::InvalidCredentials);
        }
        SessionAuthenticationRepository::record_password_proof(
            &mut tx,
            actor,
            &proof,
            authenticated_at,
        )
        .await?;
        tx.commit()
            .await
            .map_err(|_| SessionError::CommitUncertain)?;
        Ok(())
    }

    pub async fn refresh(&self, refresh_secret: &str) -> Result<TokenResponse, SessionError> {
        let refresh_token_hash =
            refresh_hash(refresh_secret).ok_or(SessionError::InvalidRefresh)?;
        let (account_id, session_id) =
            Refresh::identity(&self.repository.pool, &refresh_token_hash)
                .await?
                .ok_or(SessionError::InvalidRefresh)?;

        let mut tx = self.repository.pool.begin().await?;

        if SessionsRepository::lock_active_account(&mut tx, account_id).await? != Some(true) {
            return Err(SessionError::InvalidRefresh);
        }

        let session = SessionsRepository::lock(&mut tx, session_id).await?;

        if !session.usable {
            return Err(SessionError::InvalidRefresh);
        }

        let refresh_token = Refresh::lock(&mut tx, &refresh_token_hash, session_id).await?;

        if refresh_token.consumed {
            SessionsRepository::revoke(&mut tx, session_id, "refresh_reuse").await?;
            // Deliberately commit the revocation before returning the HTTP-domain error.
            tx.commit()
                .await
                .map_err(|_| SessionError::CommitUncertain)?;

            return Err(SessionError::InvalidRefresh);
        }

        if !refresh_token.usable {
            return Err(SessionError::InvalidRefresh);
        }

        let response = self
            .rotate_refresh(&mut tx, &session, refresh_token.id)
            .await?;

        tx.commit()
            .await
            .map_err(|_| SessionError::CommitUncertain)?;
        Ok(response)
    }

    pub async fn logout(&self, refresh_secret: &str) -> Result<(), SessionError> {
        let Some(refresh_token_hash) = refresh_hash(refresh_secret) else {
            return Ok(());
        };
        let Some((account_id, session_id)) =
            Refresh::identity(&self.repository.pool, &refresh_token_hash).await?
        else {
            return Ok(());
        };

        let mut tx = self.repository.pool.begin().await?;

        if SessionsRepository::lock_active_account(&mut tx, account_id)
            .await?
            .is_some()
        {
            SessionsRepository::lock(&mut tx, session_id).await?;
            SessionsRepository::revoke(&mut tx, session_id, "logout").await?;
        }

        tx.commit()
            .await
            .map_err(|_| SessionError::CommitUncertain)?;
        Ok(())
    }

    async fn authenticate(
        &self,
        email: &str,
        password: String,
    ) -> Result<PasswordSnapshot, SessionError> {
        // Login must also accept passwords created under earlier complexity policies.
        let password_snapshot = self.repository.password(email).await?;
        let password_matches = self
            .passwords
            .verify(
                password,
                password_snapshot
                    .as_ref()
                    .map(|credentials| credentials.password_hash.clone()),
            )
            .await
            .map_err(|error| match error {
                PasswordProcessingError::Busy => SessionError::Busy,
                PasswordProcessingError::Hashing(_) | PasswordProcessingError::Worker(_) => {
                    SessionError::Signing
                }
            })?;

        password_snapshot
            .filter(|_| password_matches)
            .ok_or(SessionError::InvalidCredentials)
    }

    pub(crate) async fn issue_tokens(
        &self,
        tx: &mut Tx<'_>,
        session: &SessionRow,
    ) -> Result<TokenResponse, SessionError> {
        let (secret, token_hash) = new_refresh()?;
        Refresh::insert(tx, session, &token_hash).await?;
        self.response(session, secret)
    }

    async fn rotate_refresh(
        &self,
        tx: &mut Tx<'_>,
        session: &SessionRow,
        token_id: uuid::Uuid,
    ) -> Result<TokenResponse, SessionError> {
        let (refresh_secret, next_token_hash) = new_refresh()?;
        let next_token_id = Refresh::insert(tx, session, &next_token_hash).await?;
        Refresh::consume(tx, token_id, next_token_id).await?;

        self.response(session, refresh_secret)
    }

    fn response(
        &self,
        session: &SessionRow,
        refresh_token: String,
    ) -> Result<TokenResponse, SessionError> {
        let (access_token, access_expires_in) = self.signer.sign(&SessionContext {
            account_id: session.account_id,
            session_id: session.id,
            login_at: session
                .login_at
                .try_into()
                .map_err(|_| SessionError::Signing)?,
            expires_at: session
                .expires_at
                .try_into()
                .map_err(|_| SessionError::Signing)?,
        })?;

        Ok(TokenResponse {
            access_token,
            refresh_token,
            token_type: "Bearer",
            access_expires_in,
            session_expires_at: session.expires_text.clone(),
            sid: session.id,
            account_id: session.account_id,
        })
    }
}

fn validate_login(request: &LoginRequest, email: &str) -> Result<(), SessionError> {
    if !is_valid_email_format(email)
        || request.password.chars().count() > 128
        || request
            .device_name
            .as_ref()
            .is_some_and(|device_name| device_name.chars().count() > 120)
    {
        return Err(SessionError::Malformed);
    }

    Ok(())
}

fn new_refresh() -> Result<(String, [u8; 32]), SessionError> {
    let mut bytes = zeroize::Zeroizing::new([0u8; 32]);

    OsRng
        .try_fill_bytes(bytes.as_mut())
        .map_err(|_| SessionError::Signing)?;

    Ok((
        URL_SAFE_NO_PAD.encode(*bytes),
        Sha256::digest(*bytes).into(),
    ))
}

pub(crate) fn refresh_hash(refresh_secret: &str) -> Option<[u8; 32]> {
    let bytes = zeroize::Zeroizing::new(URL_SAFE_NO_PAD.decode(refresh_secret).ok()?);

    if bytes.len() != 32 || URL_SAFE_NO_PAD.encode(&bytes) != refresh_secret {
        return None;
    }

    Some(Sha256::digest(&*bytes).into())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refresh_secrets_are_random_and_digest_is_of_decoded_bytes() {
        let (first, hash) = new_refresh().unwrap();
        let (second, _) = new_refresh().unwrap();
        assert_ne!(first, second);
        assert_eq!(first.len(), 43);
        assert_eq!(refresh_hash(&first), Some(hash));
        assert!(refresh_hash("invalid").is_none());
    }
}
