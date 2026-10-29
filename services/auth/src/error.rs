use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    RequestTooLarge,
    Database(sqlx::Error),
    Conflict(String),
    Validation(String),
    InternalServerError(String),
    Busy,
    RateLimited { retry_after_secs: u64 },
    RateLimitUnavailable,
}

#[derive(Debug, Serialize)]
struct ProblemDetails {
    #[serde(rename = "type")]
    problem_type: &'static str,
    title: &'static str,
    status: u16,
    detail: String,
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let retry_after_secs = match &self {
            Self::RateLimited { retry_after_secs } => Some((*retry_after_secs).max(1)),
            Self::Busy => Some(60),
            _ => None,
        };

        let (status, problem) = match self {
            Self::RequestTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                ProblemDetails {
                    problem_type: "about:blank",
                    title: "Payload Too Large",
                    status: 413,
                    detail: "Request body exceeds the configured limit".into(),
                },
            ),
            Self::Database(error) => {
                tracing::error!(error = ?error, "database error");

                let status = if matches!(
                    &error,
                    sqlx::Error::PoolTimedOut | sqlx::Error::PoolClosed | sqlx::Error::Io(_)
                ) {
                    StatusCode::SERVICE_UNAVAILABLE
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                };

                (
                    status,
                    ProblemDetails {
                        problem_type: "about:blank",
                        title: "Database error",
                        status: status.as_u16(),
                        detail: "The service cannot complete the database operation.".to_string(),
                    },
                )
            }
            Self::Conflict(detail) => {
                let status = StatusCode::CONFLICT;

                (
                    status,
                    ProblemDetails {
                        problem_type: "about:blank",
                        title: "Conflict",
                        status: status.as_u16(),
                        detail,
                    },
                )
            }
            Self::Validation(detail) => {
                let status = StatusCode::BAD_REQUEST;

                (
                    status,
                    ProblemDetails {
                        problem_type: "about:blank",
                        title: "Invalid request",
                        status: status.as_u16(),
                        detail,
                    },
                )
            }
            Self::Busy => {
                let status = StatusCode::TOO_MANY_REQUESTS;
                (
                    status,
                    ProblemDetails {
                        problem_type: "about:blank",
                        title: "Too many requests",
                        status: status.as_u16(),
                        detail: "Please retry later.".into(),
                    },
                )
            }
            Self::RateLimited { .. } => {
                let status = StatusCode::TOO_MANY_REQUESTS;

                (
                    status,
                    ProblemDetails {
                        problem_type: "about:blank",
                        title: "Too many requests",
                        status: status.as_u16(),
                        detail: "The request budget is exhausted. Please retry later.".into(),
                    },
                )
            }
            Self::RateLimitUnavailable => {
                let status = StatusCode::SERVICE_UNAVAILABLE;

                (
                    status,
                    ProblemDetails {
                        problem_type: "about:blank",
                        title: "Service unavailable",
                        status: status.as_u16(),
                        detail: "Request protection is temporarily unavailable.".into(),
                    },
                )
            }
            Self::InternalServerError(reason) => {
                tracing::error!(%reason, "internal error");
                let status = StatusCode::INTERNAL_SERVER_ERROR;

                (
                    status,
                    ProblemDetails {
                        problem_type: "about:blank",
                        title: "Internal server error",
                        status: status.as_u16(),
                        detail: "The service cannot complete the request.".to_string(),
                    },
                )
            }
        };

        let mut response = (status, Json(problem)).into_response();

        response.headers_mut().insert(
            axum::http::header::CONTENT_TYPE,
            axum::http::HeaderValue::from_static("application/problem+json"),
        );

        response.headers_mut().insert(
            axum::http::header::CACHE_CONTROL,
            axum::http::HeaderValue::from_static("no-store"),
        );

        if let Some(seconds) = retry_after_secs {
            let header = axum::http::HeaderValue::from_str(&seconds.to_string())
                .expect("An unsigned decimal integer is a valid header value");

            response
                .headers_mut()
                .insert(axum::http::header::RETRY_AFTER, header);
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rate_limit_returns_calculated_retry_and_problem_details() {
        let response = ApiError::RateLimited {
            retry_after_secs: 7,
        }
        .into_response();

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()["retry-after"], "7");
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
        assert_eq!(response.headers()["cache-control"], "no-store");

        let body = axum::body::to_bytes(response.into_body(), 4096)
            .await
            .unwrap();

        let problem: serde_json::Value = serde_json::from_slice(&body).unwrap();

        assert_eq!(problem["status"], 429);
        assert_eq!(problem["title"], "Too many requests");
    }

    #[test]
    fn unavailable_is_not_a_rate_limit_rejection() {
        let response = ApiError::RateLimitUnavailable.into_response();

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert!(!response.headers().contains_key("retry-after"));
        assert_eq!(
            response.headers()["content-type"],
            "application/problem+json"
        );
    }
}
