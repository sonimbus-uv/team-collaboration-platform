use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;

use crate::{error::ApiError, state::AppState};

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .with_state(state)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn health_db(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(HealthResponse { status: "ok" }))
}