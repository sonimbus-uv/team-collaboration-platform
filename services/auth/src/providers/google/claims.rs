use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use crate::federated_identity::{
    EmailOwnership, FederatedIdentityError, ProviderEmail, VerifiedIdentity,
};

use super::{unix_time, ISSUER};

#[derive(Deserialize)]
#[serde(untagged)]
enum Audience {
    Single(String),
    Multiple(Vec<String>),
}

#[derive(Deserialize)]
struct GoogleClaims {
    sub: String,
    aud: Audience,
    azp: Option<String>,
    exp: u64,
    iat: u64,
    nonce: String,
    email: Option<String>,
    #[serde(default)]
    email_verified: bool,
    hd: Option<String>,
}

pub(super) fn verify(
    token: &str,
    key: &DecodingKey,
    client_id: &str,
    expected_nonce_hash: &[u8; 32],
) -> Result<VerifiedIdentity, FederatedIdentityError> {
    let mut validation = Validation::new(Algorithm::RS256);
    validation.leeway = 0;
    validation.validate_nbf = true;
    validation.set_required_spec_claims(&["exp", "iss", "aud", "sub"]);
    validation.set_issuer(&[ISSUER, "accounts.google.com"]);
    validation.set_audience(&[client_id]);
    let claims = decode::<GoogleClaims>(token, key, &validation)
        .map_err(|_| FederatedIdentityError::InvalidIdentity)?
        .claims;
    let now = unix_time()?;
    validate_audience(&claims.aud, claims.azp.as_deref(), client_id)?;
    validate_subject_and_lifetime(&claims, now)?;
    validate_nonce(&claims.nonce, expected_nonce_hash)?;

    let email = claims
        .email
        .map(|address| provider_email(address, claims.email_verified, claims.hd.as_deref()))
        .transpose()?;

    Ok(VerifiedIdentity {
        issuer: ISSUER,
        subject: claims.sub,
        email,
        issued_at: claims.iat,
        expires_at: claims.exp,
    })
}

fn validate_audience(
    audience: &Audience,
    authorized_party: Option<&str>,
    client_id: &str,
) -> Result<(), FederatedIdentityError> {
    let multiple_audiences = match audience {
        Audience::Single(audience) if audience == client_id => false,
        Audience::Multiple(audiences) if audiences.iter().any(|audience| audience == client_id) => {
            audiences.len() > 1
        }
        _ => return Err(FederatedIdentityError::InvalidIdentity),
    };

    if (multiple_audiences && authorized_party.is_none())
        || authorized_party.is_some_and(|presenter| presenter != client_id)
    {
        return Err(FederatedIdentityError::InvalidIdentity);
    }

    Ok(())
}

fn validate_subject_and_lifetime(
    claims: &GoogleClaims,
    now: u64,
) -> Result<(), FederatedIdentityError> {
    if claims.sub.is_empty()
        || claims.sub.len() > 255
        || !claims.sub.bytes().all(|byte| byte.is_ascii_graphic())
        || claims.exp <= now
        || claims.iat > now.saturating_add(30)
        || claims.exp <= claims.iat
    {
        return Err(FederatedIdentityError::InvalidIdentity);
    }

    Ok(())
}

fn validate_nonce(nonce: &str, expected_hash: &[u8; 32]) -> Result<(), FederatedIdentityError> {
    if !super::valid_correlation_value(nonce) {
        return Err(FederatedIdentityError::InvalidIdentity);
    }

    let actual_hash = Sha256::digest(nonce.as_bytes());
    if !bool::from(actual_hash.as_slice().ct_eq(expected_hash)) {
        return Err(FederatedIdentityError::InvalidIdentity);
    }

    Ok(())
}

fn provider_email(
    address: String,
    email_verified: bool,
    hosted_domain: Option<&str>,
) -> Result<ProviderEmail, FederatedIdentityError> {
    if address.len() > 320
        || !address.is_ascii()
        || address
            .bytes()
            .any(|byte| byte.is_ascii_whitespace() || byte.is_ascii_control())
    {
        return Err(FederatedIdentityError::InvalidIdentity);
    }

    let (local, domain) = address
        .split_once('@')
        .ok_or(FederatedIdentityError::InvalidIdentity)?;
    if local.is_empty() || domain.is_empty() || domain.contains('@') {
        return Err(FederatedIdentityError::InvalidIdentity);
    }

    let provider_verified = email_verified
        && (domain.eq_ignore_ascii_case("gmail.com")
            || hosted_domain.is_some_and(valid_hosted_domain));
    let ownership = if provider_verified {
        EmailOwnership::ProviderVerified
    } else {
        EmailOwnership::Unverified
    };

    Ok(ProviderEmail { address, ownership })
}

fn valid_hosted_domain(domain: &str) -> bool {
    !domain.is_empty()
        && domain.len() <= 253
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
}
