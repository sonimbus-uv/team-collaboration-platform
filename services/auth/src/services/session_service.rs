use auth_service::tokens::{claims::SessionContext, issuer::AccessTokenIssuer};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use rand::{rngs::OsRng, RngCore};
use sha2::{Digest, Sha256};
use std::sync::Arc;

use crate::{
    dto::sessions::{LoginRequest, TokenResponse},
    repositories::{
        refresh_tokens_repository::RefreshTokensRepository as Refresh,
        sessions_repository::{SessionRow, SessionsRepository},
    },
    services::account_service::{AuthService, PasswordCheckError},
    validators::auth_validator::is_valid_email_format,
};

#[derive(Debug)]
pub enum SessionError {
    Malformed,
    InvalidCredentials,
    InvalidRefresh,
    Busy,
    Signing,
    Database(sqlx::Error),
    CommitUncertain,
}
impl From<sqlx::Error> for SessionError {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
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
    repo: SessionsRepository,
    passwords: AuthService,
    signer: Arc<dyn AccessSigner>,
}

impl SessionService {
    pub fn new(
        repo: SessionsRepository,
        passwords: AuthService,
        signer: Arc<dyn AccessSigner>,
    ) -> Self {
        Self {
            repo,
            passwords,
            signer,
        }
    }

    pub async fn login(&self, request: LoginRequest) -> Result<TokenResponse, SessionError> {
        let email = request.email.trim().to_lowercase();
        if !is_valid_email_format(&email)
            || request.password.chars().count() > 128
            || request
                .device_name
                .as_ref()
                .is_some_and(|v| v.chars().count() > 120)
        {
            return Err(SessionError::Malformed);
        }
        // Do not trim/normalize passwords or apply new registration complexity policy to login.
        let snapshot = self.repo.password(&email).await?;
        let valid = self
            .passwords
            .check_password(
                request.password,
                snapshot.as_ref().map(|p| p.password_hash.clone()),
            )
            .await
            .map_err(|error| match error {
                PasswordCheckError::Busy => SessionError::Busy,
                PasswordCheckError::Failed => SessionError::Signing,
            })?;
        let snapshot = snapshot
            .filter(|_| valid)
            .ok_or(SessionError::InvalidCredentials)?;
        let mut tx = self.repo.pool.begin().await?;
        if SessionsRepository::lock_account(&mut tx, snapshot.account_id).await? != Some(true)
            || !SessionsRepository::password_unchanged(&mut tx, &snapshot).await?
        {
            return Err(SessionError::InvalidCredentials);
        }
        let session = SessionsRepository::open(
            &mut tx,
            snapshot.account_id,
            request.client_type.as_str(),
            request.device_name.as_deref(),
        )
        .await?;
        let (secret, hash) = new_refresh()?;
        Refresh::insert(&mut tx, &session, &hash).await?;
        let response = self.response(&session, secret)?;
        tx.commit()
            .await
            .map_err(|_| SessionError::CommitUncertain)?;
        Ok(response)
    }

    pub async fn refresh(&self, secret: &str) -> Result<TokenResponse, SessionError> {
        let hash = refresh_hash(secret).ok_or(SessionError::InvalidRefresh)?;
        let (account, sid) = Refresh::identity(&self.repo.pool, &hash)
            .await?
            .ok_or(SessionError::InvalidRefresh)?;
        let mut tx = self.repo.pool.begin().await?;
        if SessionsRepository::lock_account(&mut tx, account).await? != Some(true) {
            return Err(SessionError::InvalidRefresh);
        }
        let session = SessionsRepository::lock(&mut tx, sid).await?;
        if !session.usable {
            return Err(SessionError::InvalidRefresh);
        }
        let token = Refresh::lock(&mut tx, &hash, sid).await?;
        if token.consumed {
            SessionsRepository::revoke(&mut tx, sid, "refresh_reuse").await?;
            // Deliberately commit the revocation before returning the HTTP-domain error.
            tx.commit()
                .await
                .map_err(|_| SessionError::CommitUncertain)?;
            return Err(SessionError::InvalidRefresh);
        }
        if !token.usable {
            return Err(SessionError::InvalidRefresh);
        }
        let (secret, next_hash) = new_refresh()?;
        let next = Refresh::insert(&mut tx, &session, &next_hash).await?;
        Refresh::consume(&mut tx, token.id, next).await?;
        let response = self.response(&session, secret)?;
        tx.commit()
            .await
            .map_err(|_| SessionError::CommitUncertain)?;
        Ok(response)
    }

    pub async fn logout(&self, secret: &str) -> Result<(), SessionError> {
        let Some(hash) = refresh_hash(secret) else {
            return Ok(());
        };
        let Some((account, sid)) = Refresh::identity(&self.repo.pool, &hash).await? else {
            return Ok(());
        };
        let mut tx = self.repo.pool.begin().await?;
        if SessionsRepository::lock_account(&mut tx, account)
            .await?
            .is_some()
        {
            SessionsRepository::lock(&mut tx, sid).await?;
            SessionsRepository::revoke(&mut tx, sid, "logout").await?;
        }
        tx.commit()
            .await
            .map_err(|_| SessionError::CommitUncertain)?;
        Ok(())
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
pub(crate) fn refresh_hash(secret: &str) -> Option<[u8; 32]> {
    let bytes = zeroize::Zeroizing::new(URL_SAFE_NO_PAD.decode(secret).ok()?);
    if bytes.len() != 32 || URL_SAFE_NO_PAD.encode(&bytes) != secret {
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
