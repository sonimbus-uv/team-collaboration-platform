use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

#[derive(Debug)]
pub enum ApiError {
    Database(sqlx::Error),
}

#[derive(Debug, Serialize)]
struct ProblemDetails {
    #[serde(rename = "type")]
    problem_type: &'static str,
    title: &'static str,
    status: u16,
    detail: &'static str,
}

impl From<sqlx::Error> for ApiError {
    fn from(error: sqlx::Error) -> Self {
        Self::Database(error)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, problem) = match self {
            Self::Database(_) => {
                let status = StatusCode::SERVICE_UNAVAILABLE;

                (
                    status,
                    ProblemDetails {
                        problem_type: "about:blank",
                        title: "Database unavailable",
                        status: status.as_u16(),
                        detail: "The service cannot connect to the database.",
                    },
                )
            }
        };

        (status, Json(problem)).into_response()
    }
}