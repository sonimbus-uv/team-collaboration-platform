use axum::{
    http::{header, HeaderValue, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

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

impl std::fmt::Debug for ApiError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let kind = match self {
            Self::RequestTooLarge => "RequestTooLarge",
            Self::Database(_) => "Database",
            Self::Conflict(_) => "Conflict",
            Self::Validation(_) => "Validation",
            Self::InternalServerError(_) => "InternalServerError",
            Self::Busy => "Busy",
            Self::RateLimited { .. } => "RateLimited",
            Self::RateLimitUnavailable => "RateLimitUnavailable",
        };
        formatter.write_str(kind)
    }
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

        let (status, title, detail) = match self {
            Self::RequestTooLarge => (
                StatusCode::PAYLOAD_TOO_LARGE,
                "Payload Too Large",
                "Request body exceeds the configured limit".into(),
            ),
            Self::Database(error) => {
                let unavailable = matches!(
                    error,
                    sqlx::Error::PoolTimedOut
                        | sqlx::Error::PoolClosed
                        | sqlx::Error::Io(_)
                        | sqlx::Error::Tls(_)
                );
                tracing::error!(unavailable, "Database operation failed");
                let status = if unavailable {
                    StatusCode::SERVICE_UNAVAILABLE
                } else {
                    StatusCode::INTERNAL_SERVER_ERROR
                };
                (
                    status,
                    "Database error",
                    "The service cannot complete the database operation.".into(),
                )
            }
            Self::Conflict(detail) => (StatusCode::CONFLICT, "Conflict", detail),
            Self::Validation(detail) => (StatusCode::BAD_REQUEST, "Invalid request", detail),
            Self::Busy => (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many requests",
                "Please retry later.".into(),
            ),
            Self::RateLimited { .. } => (
                StatusCode::TOO_MANY_REQUESTS,
                "Too many requests",
                "The request budget is exhausted. Please retry later.".into(),
            ),
            Self::RateLimitUnavailable => (
                StatusCode::SERVICE_UNAVAILABLE,
                "Service unavailable",
                "Request protection is temporarily unavailable.".into(),
            ),
            Self::InternalServerError(reason) => {
                tracing::error!(
                    has_reason = !reason.is_empty(),
                    "Internal request processing failed"
                );
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Internal server error",
                    "The service cannot complete the request.".into(),
                )
            }
        };
        let problem = ProblemDetails {
            problem_type: "about:blank",
            title,
            status: status.as_u16(),
            detail,
        };

        let mut response = (status, Json(problem)).into_response();

        response.headers_mut().insert(
            header::CONTENT_TYPE,
            HeaderValue::from_static("application/problem+json"),
        );

        response
            .headers_mut()
            .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));

        if let Some(seconds) = retry_after_secs {
            let header = HeaderValue::from_str(&seconds.to_string())
                .expect("An unsigned decimal integer is a valid header value");

            response.headers_mut().insert(header::RETRY_AFTER, header);
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Default)]
    struct CapturedLogs(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

    impl std::io::Write for CapturedLogs {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            self.0.lock().unwrap().extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    #[tokio::test]
    async fn internal_diagnostics_are_absent_from_debug_logs_and_http_bodies() {
        let logs = CapturedLogs::default();
        let writer = logs.clone();
        let subscriber = tracing_subscriber::fmt()
            .without_time()
            .with_ansi(false)
            .with_writer(move || writer.clone())
            .finish();
        let responses = tracing::subscriber::with_default(subscriber, || {
            [
                ApiError::Database(sqlx::Error::Protocol("private-database-diagnostic".into())),
                ApiError::InternalServerError("private-internal-diagnostic".into()),
            ]
            .map(|error| {
                let debug = format!("{error:?}");
                assert!(!debug.contains("private-"));
                error.into_response()
            })
        });
        for response in responses {
            assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
            let body = axum::body::to_bytes(response.into_body(), 4096)
                .await
                .unwrap();
            assert!(!String::from_utf8_lossy(&body).contains("private-"));
        }
        let output = logs.0.lock().unwrap();
        let output = String::from_utf8_lossy(&output);
        assert!(output.contains("Database operation failed"));
        assert!(output.contains("Internal request processing failed"));
        assert!(!output.contains("private-"));
    }

    #[test]
    fn database_transport_failures_are_service_unavailable_without_retry_advice() {
        let tls_error = sqlx::Error::Tls(Box::new(std::io::Error::other("private-tls-diagnostic")));
        for error in [
            sqlx::Error::PoolClosed,
            sqlx::Error::PoolTimedOut,
            tls_error,
        ] {
            let response = ApiError::Database(error).into_response();
            assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
            assert!(!response.headers().contains_key(header::RETRY_AFTER));
        }
    }

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
