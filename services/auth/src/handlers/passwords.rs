use crate::{
    dto::passwords::{ChangePassword, ForgotPassword, ResetPassword},
    services::password_service::PasswordError,
    state::AppState,
};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

fn password_error_response(error: PasswordError) -> Response {
    let (status, kind) = match error {
        PasswordError::Malformed => (StatusCode::BAD_REQUEST, "malformed-request"),
        PasswordError::InvalidPassword => (StatusCode::BAD_REQUEST, "invalid-password"),
        PasswordError::InvalidCurrent => (StatusCode::BAD_REQUEST, "invalid-current-password"),
        PasswordError::InvalidCode => (StatusCode::BAD_REQUEST, "invalid-recovery-code"),
        PasswordError::InvalidAccess => (StatusCode::UNAUTHORIZED, "invalid-access-token"),
        PasswordError::NotAllowed => (StatusCode::CONFLICT, "password-change-not-allowed"),
        PasswordError::Unchanged => (StatusCode::CONFLICT, "password-unchanged"),
        PasswordError::Changed => (StatusCode::CONFLICT, "credentials-changed"),
        PasswordError::Limited(seconds) => {
            return super::response::protection(crate::error::ApiError::RateLimited {
                retry_after_secs: seconds,
            })
        }
        PasswordError::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "auth-unavailable"),
        PasswordError::Internal | PasswordError::CommitUncertain => {
            (StatusCode::INTERNAL_SERVER_ERROR, "internal-error")
        }
    };
    super::response::problem(status, kind)
}

fn json_error(rejection: JsonRejection) -> Response {
    if rejection.status() == StatusCode::PAYLOAD_TOO_LARGE {
        super::response::problem(StatusCode::PAYLOAD_TOO_LARGE, "request-too-large")
    } else {
        password_error_response(PasswordError::Malformed)
    }
}

pub async fn change(
    State(state): State<AppState>,
    headers: HeaderMap,
    input: Result<Json<ChangePassword>, JsonRejection>,
) -> Response {
    let Some(service) = state.password_service else {
        return password_error_response(PasswordError::Unavailable);
    };
    let mut authorization_values = headers.get_all(axum::http::header::AUTHORIZATION).iter();
    let value = authorization_values
        .next()
        .and_then(|value| value.to_str().ok());
    if authorization_values.next().is_some() {
        return password_error_response(PasswordError::InvalidAccess);
    }
    let claims = value
        .and_then(|value| value.split_once(' '))
        .filter(|(scheme, _)| scheme.eq_ignore_ascii_case("Bearer"))
        .and_then(|(_, token)| state.token_issuer.verify_for_auth(token).ok());
    let Some(claims) = claims else {
        return password_error_response(PasswordError::InvalidAccess);
    };
    let Json(input) = match input {
        Ok(request) => request,
        Err(rejection) => return json_error(rejection),
    };
    match service.change(claims.sub, claims.sid, input).await {
        Ok(()) => super::response::protected(StatusCode::NO_CONTENT.into_response()),
        Err(error) => password_error_response(error),
    }
}

pub async fn forgot(
    State(state): State<AppState>,
    input: Result<Json<ForgotPassword>, JsonRejection>,
) -> Response {
    let Some(service) = state.password_service else {
        return password_error_response(PasswordError::Unavailable);
    };
    let Json(input) = match input {
        Ok(request) => request,
        Err(rejection) => return json_error(rejection),
    };
    match service.forgot(input.email).await {
        Ok(accepted) => {
            super::response::protected((StatusCode::ACCEPTED, Json(accepted)).into_response())
        }
        Err(error) => password_error_response(error),
    }
}

pub async fn reset(
    State(state): State<AppState>,
    input: Result<Json<ResetPassword>, JsonRejection>,
) -> Response {
    let Some(service) = state.password_service else {
        return password_error_response(PasswordError::Unavailable);
    };
    let Json(input) = match input {
        Ok(request) => request,
        Err(rejection) => return json_error(rejection),
    };
    match service.reset(input).await {
        Ok(()) => super::response::protected(StatusCode::NO_CONTENT.into_response()),
        Err(error) => password_error_response(error),
    }
}
