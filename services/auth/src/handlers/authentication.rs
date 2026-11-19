use auth_service::{
    session_check_service::SessionCheckService, session_check_types::SessionCheckRequest,
};
use axum::{
    http::{header, HeaderMap, StatusCode},
    response::Response,
};

use crate::{repositories::session_authentication_repository::SessionActor, state::AppState};

pub enum AccessError {
    Invalid,
    Unavailable,
}

pub async fn authenticated_actor(
    state: &AppState,
    headers: &HeaderMap,
) -> Result<SessionActor, AccessError> {
    let token = bearer_token(headers).ok_or(AccessError::Invalid)?;
    let claims = state
        .token_issuer
        .verify_for_auth(token)
        .map_err(|_| AccessError::Invalid)?;
    let session_status = SessionCheckService::new(state.pool.clone())
        .check(SessionCheckRequest {
            sid: claims.sid,
            account_id: claims.sub,
        })
        .await
        .map_err(|_| AccessError::Unavailable)?;

    if !session_status.active {
        return Err(AccessError::Invalid);
    }

    Ok(SessionActor {
        account_id: claims.sub,
        session_id: claims.sid,
    })
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let mut authorization_headers = headers.get_all(header::AUTHORIZATION).iter();
    let credentials = authorization_headers.next()?.to_str().ok()?;
    if authorization_headers.next().is_some() {
        return None;
    }

    let (scheme, token) = credentials.split_once(' ')?;
    scheme.eq_ignore_ascii_case("Bearer").then_some(token)
}

pub fn access_error(error: AccessError) -> Response {
    match error {
        AccessError::Invalid => {
            super::response::problem(StatusCode::UNAUTHORIZED, "invalid-access-token")
        }
        AccessError::Unavailable => {
            super::response::problem(StatusCode::SERVICE_UNAVAILABLE, "auth-unavailable")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bearer_credentials_require_one_header_and_the_correct_scheme() {
        let mut headers = HeaderMap::new();
        assert!(bearer_token(&headers).is_none());

        headers.insert(header::AUTHORIZATION, "Basic credentials".parse().unwrap());
        assert!(bearer_token(&headers).is_none());

        headers.insert(header::AUTHORIZATION, "bEaReR token".parse().unwrap());
        assert_eq!(bearer_token(&headers), Some("token"));

        headers.append(header::AUTHORIZATION, "Bearer other".parse().unwrap());
        assert!(bearer_token(&headers).is_none());
    }
}
