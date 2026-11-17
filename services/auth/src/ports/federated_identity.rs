use std::fmt;

pub struct AuthorizationRequest<'a> {
    pub state: &'a str,
    pub nonce: &'a str,
}

pub struct AuthorizationCode<'a> {
    pub code: &'a str,
    pub expected_nonce_hash: &'a [u8; 32],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EmailOwnership {
    Unverified,
    ProviderVerified,
}

pub struct ProviderEmail {
    pub address: String,
    /// Provider proof does not bypass the database's exclusive contact assignment.
    pub ownership: EmailOwnership,
}

pub struct VerifiedIdentity {
    pub issuer: &'static str,
    pub subject: String,
    pub email: Option<ProviderEmail>,
    pub issued_at: u64,
    pub expires_at: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FederatedIdentityError {
    Configuration,
    InvalidRequest,
    AuthorizationRejected,
    InvalidIdentity,
    Unavailable,
}

impl fmt::Display for FederatedIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Configuration => "Invalid federated identity configuration",
            Self::InvalidRequest => "Invalid federated authorization request",
            Self::AuthorizationRejected => "Federated authorization was rejected",
            Self::InvalidIdentity => "Federated identity validation failed",
            Self::Unavailable => "Federated identity provider unavailable",
        })
    }
}

impl std::error::Error for FederatedIdentityError {}

#[async_trait::async_trait]
pub trait FederatedIdentityProvider: Send + Sync {
    fn authorization_url(
        &self,
        request: AuthorizationRequest<'_>,
    ) -> Result<String, FederatedIdentityError>;

    async fn exchange_code(
        &self,
        authorization: AuthorizationCode<'_>,
    ) -> Result<VerifiedIdentity, FederatedIdentityError>;
}
