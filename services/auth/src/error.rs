use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    Database(sqlx::Error),
    Conflict(String),
    Validation(String),
    InternalServerError(String),
    Busy,
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
        let (status, problem) = match self {
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
        if status == StatusCode::TOO_MANY_REQUESTS {
            response
                .headers_mut()
                .insert("retry-after", axum::http::HeaderValue::from_static("60"));
        }
        response
    }
}
