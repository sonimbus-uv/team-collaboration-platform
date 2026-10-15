use axum::{
    extract::Request,
    middleware::{self, Next},
    response::{IntoResponse, Response},
};
use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use crate::{error::ApiError, handlers::auth, state::AppState};

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn router(state: AppState) -> Router {
    let limiter = Arc::new(Mutex::new((Instant::now(), 0u32)));
    let auth_routes = Router::new()
        .route("/auth/register", post(auth::register))
        .route(
            "/auth/verify-email",
            get(auth::verify_email_page)
                // Axum otherwise dispatches HEAD to the GET handler.
                .head(|| async {
                    crate::email::verification_page::verification_page_response(
                        axum::http::StatusCode::METHOD_NOT_ALLOWED,
                        String::new(),
                    )
                })
                .post(auth::verify_email),
        )
        .route("/auth/resend-verification", post(auth::resend_verification))
        .route_layer(middleware::from_fn_with_state(limiter, limit_auth_requests));
    Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .merge(auth_routes)
        .with_state(state)
}

async fn limit_auth_requests(
    State(limiter): State<Arc<Mutex<(Instant, u32)>>>,
    request: Request,
    next: Next,
) -> Response {
    let allowed = {
        let mut window = limiter.lock().expect("rate limiter mutex poisoned");
        if window.0.elapsed() >= Duration::from_secs(60) {
            *window = (Instant::now(), 0);
        }
        if window.1 >= 120 {
            false
        } else {
            window.1 += 1;
            true
        }
    };
    if allowed {
        next.run(request).await
    } else {
        if request.method() == axum::http::Method::GET
            && request.uri().path() == "/auth/verify-email"
        {
            let mut response = crate::email::verification_page::verification_page_response(
                axum::http::StatusCode::TOO_MANY_REQUESTS,
                crate::email::verification_page::verification_unavailable_page(),
            );
            response
                .headers_mut()
                .insert("retry-after", axum::http::HeaderValue::from_static("60"));
            return response;
        }
        ApiError::Busy.into_response()
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;
    #[tokio::test]
    async fn rate_limit_returns_retry_after() {
        let limiter = Arc::new(Mutex::new((Instant::now(), 120u32)));
        let app = Router::new()
            .route("/test", get(|| async { "ok" }))
            .route_layer(middleware::from_fn_with_state(limiter, limit_auth_requests));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/test")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), axum::http::StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()["retry-after"], "60");
    }
}
