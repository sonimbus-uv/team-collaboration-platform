use crate::error::ApiError;
use axum::{
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    Json,
};

pub(crate) fn protection(error: ApiError) -> Response {
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

pub(crate) fn problem(status: StatusCode, kind: &str) -> Response {
    let body = serde_json::json!(
        {
            "type": format!("/problems/{kind}"),
            "title": status.canonical_reason().unwrap_or("Error"),
            "status": status.as_u16(),
            "detail": "The authentication operation could not be completed."
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

pub(crate) fn protected(mut response: Response) -> Response {
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
