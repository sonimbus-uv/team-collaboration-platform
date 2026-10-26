use crate::{
    dto::sessions::{LoginRequest, RefreshRequest},
    error::ApiError,
    protection::rate_limit::RateLimitDecision,
    services::session_service::SessionError,
    state::AppState,
};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

pub async fn login(
    State(state): State<AppState>,
    request: Result<Json<LoginRequest>, JsonRejection>,
) -> Response {
    if !state.login_enabled {
        return problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "auth-unavailable");
    }

    let Ok(Json(request)) = request else {
        return from_error(SessionError::Malformed);
    };

    match state
        .auth_rate_limiter
        .check_login_account(&request.email)
        .await
    {
        Ok(RateLimitDecision::Allowed) => {}
        Ok(RateLimitDecision::Limited { retry_after_secs }) => {
            return protection(ApiError::RateLimited { retry_after_secs })
        }
        Err(_) => return problem(
            StatusCode::SERVICE_UNAVAILABLE,
            "auth-unavailable"),
    }

    match state.sessions().login(request).await {
        Ok(tokens) => protected(Json(tokens).into_response()),
        Err(error) => from_error(error),
    }
}

pub async fn refresh(
    State(state): State<AppState>,
    request: Result<Json<RefreshRequest>, JsonRejection>,
) -> Response {
    let request = match token_request(request) {
        Ok(r) => r,
        Err(error) => return from_error(error),
    };

    match state.sessions().refresh(&request.refresh_token).await {
        Ok(tokens) => protected(Json(tokens).into_response()),
        Err(error) => from_error(error),
    }
}

pub async fn logout(
    State(state): State<AppState>,
    request: Result<Json<RefreshRequest>, JsonRejection>,
) -> Response {
    let request = match token_request(request) {
        Ok(r) => r,
        Err(error) => return from_error(error),
    };

    match state.sessions().logout(&request.refresh_token).await {
        Ok(()) => protected(StatusCode::NO_CONTENT.into_response()),
        Err(error) => from_error(error),
    }
}

fn token_request(
    request: Result<Json<RefreshRequest>, JsonRejection>,
) -> Result<RefreshRequest, SessionError> {
    match request {
        Ok(Json(r)) if !r.refresh_token.is_empty() && r.refresh_token.chars().count() <= 512 => {
            Ok(r)
        }
        _ => Err(SessionError::Malformed),
    }
}

pub fn from_error(error: SessionError) -> Response {
    let (status, kind) = match error {
        SessionError::Malformed => (
            StatusCode::BAD_REQUEST,
            "malformed-request"
        ),
        SessionError::InvalidCredentials => (
            StatusCode::UNAUTHORIZED,
            "invalid-credentials"
        ),
        SessionError::InvalidRefresh => (
            StatusCode::UNAUTHORIZED,
            "invalid-refresh-token"
        ),
        SessionError::Busy => return protection(ApiError::Busy),
        SessionError::CommitUncertain => (
            StatusCode::INTERNAL_SERVER_ERROR,
            "internal-error"
        ),
        SessionError::Signing => (
            StatusCode::SERVICE_UNAVAILABLE,
            "auth-unavailable"
        ),
        SessionError::Database(error) => {
            if matches!(
                error,
                sqlx::Error::PoolTimedOut
                    | sqlx::Error::PoolClosed
                    | sqlx::Error::Io(_)
                    | sqlx::Error::Tls(_)
            ) {
                (StatusCode::SERVICE_UNAVAILABLE,
                    "auth-unavailable"
                )
            } else {
                (StatusCode::INTERNAL_SERVER_ERROR,
                    "internal-error"
                )
            }
        }
    };
    // Intentionally no Debug logging of database errors, requests or token responses.
    problem(status, kind)
}

pub fn protection(error: ApiError) -> Response {
    let original = error.into_response();
    let status = original.status();
    let kind = match status {
        StatusCode::TOO_MANY_REQUESTS => "too-many-requests",
        StatusCode::SERVICE_UNAVAILABLE => "auth-unavailable",
        _ => "internal-error",
    };

    let mut response = problem(status, kind);

    if let Some(value) = original.headers().get(header::RETRY_AFTER) {
        response
            .headers_mut()
            .insert(header::RETRY_AFTER, value.clone());
    }

    response
}

fn problem(status: StatusCode, kind: &str) -> Response {
    let body = serde_json::json!(
        {
            "type":format!("/problems/{kind}"),
            "title":status.canonical_reason().unwrap_or("Error"),
            "status":status.as_u16(),
            "detail":"The authentication operation could not be completed."
        }
    );

    let mut response = protected((status, Json(body)).into_response());

    response.headers_mut().insert(
        header::CONTENT_TYPE,
        header::HeaderValue::from_static("application/problem+json"),
    );

    if status == StatusCode::UNAUTHORIZED {
        response.headers_mut().insert(
            header::WWW_AUTHENTICATE,
            header::HeaderValue::from_static("Bearer"),
        );
    }

    response
}

fn protected(mut response: Response) -> Response {
    response.headers_mut().insert(
        header::CACHE_CONTROL,
        header::HeaderValue::from_static("no-store"),
    );

    response
        .headers_mut()
        .insert(header::PRAGMA, header::HeaderValue::from_static("no-cache"));

    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        header::HeaderValue::from_static("nosniff"),
    );

    response
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn protection_errors_keep_contract_types_and_retry_without_echoing_credentials() {
        for (error, status, kind) in [
            (
                ApiError::RateLimited {
                    retry_after_secs: 7,
                },
                StatusCode::TOO_MANY_REQUESTS,
                "too-many-requests",
            ),
            (
                ApiError::RateLimitUnavailable,
                StatusCode::SERVICE_UNAVAILABLE,
                "auth-unavailable",
            ),
        ] {
            let response = protection(error);

            assert_eq!(response.status(), status);
            assert_eq!(response.headers()[header::CACHE_CONTROL], "no-store");

            if status == StatusCode::TOO_MANY_REQUESTS {
                assert_eq!(response.headers()[header::RETRY_AFTER], "7");
            }

            let body = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap();
            let value: serde_json::Value = serde_json::from_slice(&body).unwrap();

            assert_eq!(value["type"], format!("/problems/{kind}"));
        }
    }

    #[test]
    fn ambiguous_commit_is_never_a_safe_503() {
        assert_eq!(
            from_error(SessionError::CommitUncertain).status(),
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
}
