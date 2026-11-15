use axum::{
    extract::{rejection::JsonRejection, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

use super::{
    authentication::{access_error, authenticated_actor},
    response::{json_error, problem, protected, rate_limited},
};
use crate::{
    dto::contact_email::{
        RequestContactVerification, ResendContactVerification, VerifyContactEmail,
    },
    services::contact_email_service::{ContactEmailError, ContactRequestResult},
    state::AppState,
};

pub async fn read(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(service) = &state.contact_email_service else {
        return contact_error_response(ContactEmailError::Unavailable);
    };

    let actor = match authenticated_actor(&state, &headers).await {
        Ok(actor) => actor,
        Err(error) => return access_error(error),
    };

    match service.snapshot(actor).await {
        Ok(contact) => protected(Json(contact).into_response()),
        Err(error) => contact_error_response(error),
    }
}

pub async fn start(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<RequestContactVerification>, JsonRejection>,
) -> Response {
    let Some(service) = &state.contact_email_service else {
        return contact_error_response(ContactEmailError::Unavailable);
    };

    let actor = match authenticated_actor(&state, &headers).await {
        Ok(actor) => actor,
        Err(error) => return access_error(error),
    };

    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => return json_error(error),
    };

    match service.start(actor, request.email).await {
        Ok(ContactRequestResult::Accepted(attempt)) => {
            protected((StatusCode::ACCEPTED, Json(attempt)).into_response())
        }
        Ok(ContactRequestResult::AlreadyVerified) => {
            protected(StatusCode::NO_CONTENT.into_response())
        }
        Err(error) => contact_error_response(error),
    }
}

pub async fn resend(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<ResendContactVerification>, JsonRejection>,
) -> Response {
    let Some(service) = &state.contact_email_service else {
        return contact_error_response(ContactEmailError::Unavailable);
    };

    let actor = match authenticated_actor(&state, &headers).await {
        Ok(actor) => actor,
        Err(error) => return access_error(error),
    };

    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => return json_error(error),
    };

    match service.resend(actor, request.verification_attempt_id).await {
        Ok(attempt) => protected((StatusCode::ACCEPTED, Json(attempt)).into_response()),
        Err(error) => contact_error_response(error),
    }
}

pub async fn verify(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<VerifyContactEmail>, JsonRejection>,
) -> Response {
    let Some(service) = &state.contact_email_service else {
        return contact_error_response(ContactEmailError::Unavailable);
    };

    let actor = match authenticated_actor(&state, &headers).await {
        Ok(actor) => actor,
        Err(error) => return access_error(error),
    };

    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => return json_error(error),
    };

    match service
        .verify(actor, request.verification_attempt_id, request.code)
        .await
    {
        Ok(()) => protected(StatusCode::NO_CONTENT.into_response()),
        Err(error) => contact_error_response(error),
    }
}

fn contact_error_response(error: ContactEmailError) -> Response {
    let (status, kind) = match error {
        ContactEmailError::Malformed => (StatusCode::BAD_REQUEST, "malformed-request"),
        ContactEmailError::InvalidAccess => (StatusCode::UNAUTHORIZED, "invalid-access-token"),
        ContactEmailError::InvalidCode => (StatusCode::BAD_REQUEST, "invalid-verification-code"),
        ContactEmailError::InUse => (StatusCode::CONFLICT, "contact-email-in-use"),
        ContactEmailError::ChangeNotSupported => {
            (StatusCode::CONFLICT, "contact-email-change-not-supported")
        }
        ContactEmailError::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "auth-unavailable"),
        ContactEmailError::Database | ContactEmailError::CommitUncertain => {
            (StatusCode::INTERNAL_SERVER_ERROR, "internal-error")
        }
        ContactEmailError::Limited(retry_after_secs) => return rate_limited(retry_after_secs),
    };

    problem(status, kind)
}
