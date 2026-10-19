use std::{net::SocketAddr, sync::Arc};

use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header::RETRY_AFTER, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

use crate::{
    auth_rate_limiter::AuthRateLimiter,
    client_ip::resolve_client_ip,
    email::verification_page::{
        verification_page_response, 
        verification_unavailable_page,
    },
    error::ApiError,
    handlers::auth,
    rate_limit::{AuthOperation, RateLimitDecision},
    state::AppState,
};

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn router(state: AppState) -> Router {
    let limiter = state.auth_rate_limiter.clone();

    let auth_routes = Router::new()
        .route("/auth/register", post(auth::register))
        .route(
            "/auth/verify-email",
            get(auth::verify_email_page)
                .head(|| async {
                    verification_page_response(
                        StatusCode::METHOD_NOT_ALLOWED,
                        String::new(),
                    )
                })
                .post(auth::verify_email),
        )
        .route("/auth/resend-verification", post(auth::resend_verification))
        .route_layer(middleware::from_fn_with_state(limiter, limit_auth_requests,)
    );

    Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .merge(auth_routes)
        .with_state(state)
}

async fn limit_auth_requests(
    State(limiter): State<Arc<AuthRateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    let Some(operation) =
        AuthOperation::from_request(request.method(), request.uri().path())
    else {
        return next.run(request).await;
    };

    let browser_verification = request.method() == Method::GET
        && operation == AuthOperation::Verify;

    let Some(peer) = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0)
    else {
        tracing::error!("Connection information missing for protected Auth request");

        return protection_error_response(
            browser_verification,
            ApiError::RateLimitUnavailable,
        );
    };

    let client_ip = match resolve_client_ip(
        peer,
        request.headers(),
        limiter.trusted_proxy_ips(),
    ) {
        Ok(ip) => ip,
        Err(_) => {
            return protection_error_response(
                browser_verification,
                ApiError::Validation(
                    "Missing or invalid client address from trusted proxy".into(),
                ),
            );
        }
    };

    match limiter.check_ip(operation, client_ip).await {
        Ok(RateLimitDecision::Allowed) => next.run(request).await,
        Ok(RateLimitDecision::Limited { retry_after_secs }) => {
            protection_error_response(
                browser_verification,
                ApiError::RateLimited { retry_after_secs },
            )
        }
        Err(error) => {
            tracing::warn!(
                error_kind = ?error,
                "Auth rate limit check unavailable"
            );

            protection_error_response(
                browser_verification,
                ApiError::RateLimitUnavailable,
            )
        }
    }
}

fn protection_error_response(
    browser_verification: bool,
    error: ApiError,
) -> Response {
    let api_response = error.into_response();

    if !browser_verification {
        return api_response;
    }

    let mut html_response = verification_page_response(
        api_response.status(),
        verification_unavailable_page(),
    );

    if let Some(retry_after) = api_response.headers().get(RETRY_AFTER) {
        html_response
            .headers_mut()
            .insert(RETRY_AFTER, retry_after.clone());
    }

    html_response
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse { status: "ok" })
}

async fn health_db(
    State(state): State<AppState>,
) -> Result<Json<HealthResponse>, ApiError> {
    sqlx::query_scalar::<_, i32>("SELECT 1")
        .fetch_one(&state.pool)
        .await?;

    Ok(Json(HealthResponse { status: "ok" }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tower::ServiceExt;

    #[test]
    fn browser_rate_limit_preserves_html_status_and_retry_after() {
        let response = protection_error_response(
            true,
            ApiError::RateLimited {
                retry_after_secs: 7,
            },
        );

        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        assert_eq!(response.headers()["retry-after"], "7");
        assert_eq!(
            response.headers()["content-type"],
            "text/html; charset=utf-8"
        );
        assert_eq!(response.headers()["cache-control"], "no-store");
        assert_eq!(response.headers()["referrer-policy"], "no-referrer");
    }

    #[test]
    fn browser_unavailability_returns_html_without_retry_after() {
        let response =
            protection_error_response(true, ApiError::RateLimitUnavailable);

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response.headers()["content-type"],
            "text/html; charset=utf-8"
        );
        assert!(!response.headers().contains_key("retry-after"));
    }

    #[tokio::test]
    async fn head_bypasses_client_identity_and_redis() {
        let limiter = Arc::new(AuthRateLimiter::for_tests(
            "redis://127.0.0.1:1/0".into(),
        ));

        let app = Router::new()
            .route(
                "/auth/verify-email",
                get(|| async { StatusCode::OK })
                    .head(|| async { StatusCode::METHOD_NOT_ALLOWED }),
            )
            .route_layer(middleware::from_fn_with_state(
                limiter,
                limit_auth_requests,
            ));

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::HEAD)
                    .uri("/auth/verify-email")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}