use std::{net::SocketAddr, sync::Arc};

mod jwks;

use crate::{
    email::verification_page::{verification_page_response, verification_unavailable_page},
    error::ApiError,
    handlers::auth,
    protection::{
        auth_rate_limiter::AuthRateLimiter,
        client_ip::resolve_client_ip,
        rate_limit::{AuthOperation, RateLimitDecision},
    },
    state::AppState,
};
use axum::{
    extract::{ConnectInfo, Request, State},
    http::{header::RETRY_AFTER, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

#[derive(Debug, Serialize)]
struct HealthResponse {
    status: &'static str,
}

pub fn router(state: AppState) -> Router {
    let jwks_routes = jwks::router(state.token_issuer.clone());
    let rate_limiter = state.auth_rate_limiter.clone();

    let session_and_password_routes = Router::new()
        .route(
            "/api/v1/auth/change-password",
            post(crate::handlers::passwords::change),
        )
        .route(
            "/api/v1/auth/forgot-password",
            post(crate::handlers::passwords::forgot),
        )
        .route(
            "/api/v1/auth/reset-password",
            post(crate::handlers::passwords::reset),
        )
        .route(
            "/api/v1/auth/login",
            post(crate::handlers::sessions::login),
        )
        .route(
            "/api/v1/auth/refresh",
            post(crate::handlers::sessions::refresh),
        )
        .route(
            "/api/v1/auth/logout",
            post(crate::handlers::sessions::logout),
        )
        .layer(axum::extract::DefaultBodyLimit::max(4096))
        .route_layer(middleware::from_fn_with_state(
            rate_limiter.clone(),
            limit_auth_requests,
        ));

    let registration_routes = Router::new()
        .route(
            "/api/v1/auth/register",
            post(auth::register)
        )
        .route(
            "/api/v1/auth/verify-email",
            get(auth::verify_email_page)
                .head(|| async {
                    verification_page_response(StatusCode::METHOD_NOT_ALLOWED, String::new())
                })
                .post(auth::verify_email),
        )
        .route(
            "/api/v1/auth/resend-verification",
            post(auth::resend_verification),
        )
        .layer(axum::extract::DefaultBodyLimit::max(4096))
        .route_layer(middleware::from_fn_with_state(
            rate_limiter,
            limit_auth_requests,
        ))
        .layer(middleware::from_fn(auth::registration_headers));

    Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .route("/health/redis", get(health_redis))
        .merge(registration_routes)
        .merge(session_and_password_routes)
        .merge(jwks_routes)
        .with_state(state)
}

async fn limit_auth_requests(
    State(rate_limiter): State<Arc<AuthRateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    let Some(auth_operation) = AuthOperation::from_request(request.method(), request.uri().path())
    else {
        return next.run(request).await;
    };

    let respond_with_protection_error = |is_browser_request, error| {
        if auth_operation.is_session() {
            crate::handlers::sessions::protection(error)
        } else {
            protection_error_response(is_browser_request, error)
        }
    };

    let is_browser_verification =
        request.method() == Method::GET && auth_operation == AuthOperation::Verify;

    let Some(peer_address) = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|connection_info| connection_info.0)
    else {
        tracing::error!("Connection information missing for protected Auth request");

        return respond_with_protection_error(
            is_browser_verification,
            ApiError::RateLimitUnavailable,
        );
    };

    let client_ip = match resolve_client_ip(
        peer_address,
        request.headers(),
        rate_limiter.trusted_proxy_ips(),
    ) {
        Ok(resolved_ip) => resolved_ip,
        Err(_) => {
            return respond_with_protection_error(
                is_browser_verification,
                ApiError::InternalServerError(
                    "Missing or invalid client address from trusted proxy".into(),
                ),
            );
        }
    };

    match rate_limiter.check_ip(auth_operation, client_ip).await {
        Ok(RateLimitDecision::Allowed) => next.run(request).await,
        Ok(RateLimitDecision::Limited { retry_after_secs }) => respond_with_protection_error(
            is_browser_verification,
            ApiError::RateLimited { retry_after_secs },
        ),
        Err(error) if auth_operation == AuthOperation::Verify => {
            tracing::warn!(
                error_kind = ?error,
                "Auth verification admitted without Redis rate limiting"
            );
            next.run(request).await
        }
        Err(error) => {
            tracing::warn!(
                error_kind = ?error,
                "Auth rate limit check unavailable"
            );

            respond_with_protection_error(is_browser_verification, ApiError::RateLimitUnavailable)
        }
    }
}

fn protection_error_response(is_browser_verification: bool, error: ApiError) -> Response {
    let api_response = error.into_response();

    if !is_browser_verification {
        return api_response;
    }

    let mut html_response =
        verification_page_response(api_response.status(), verification_unavailable_page());

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

async fn health_redis(State(state): State<AppState>) -> Result<Json<HealthResponse>, ApiError> {
    state
        .auth_rate_limiter
        .health_check()
        .await
        .map_err(|_| ApiError::RateLimitUnavailable)?;

    Ok(Json(HealthResponse { status: "ok" }))
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

    fn protected_app(rate_limiter: AuthRateLimiter) -> Router {
        Router::new()
            .route(
                "/api/v1/auth/register",
                post(|| async { StatusCode::CREATED }),
            )
            .route(
                "/api/v1/auth/resend-verification",
                post(|| async { StatusCode::ACCEPTED }),
            )
            .route(
                "/api/v1/auth/verify-email",
                get(|| async { StatusCode::OK }).post(|| async { StatusCode::OK }),
            )
            .route_layer(middleware::from_fn_with_state(
                Arc::new(rate_limiter),
                limit_auth_requests,
            ))
    }

    fn protected_request(
        method: Method,
        path: &str,
        peer_address: SocketAddr,
        client_ip_header: &str,
    ) -> Request {
        Request::builder()
            .method(method)
            .uri(path)
            .extension(ConnectInfo(peer_address))
            .header("x-real-ip", client_ip_header)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn redis_failure_allows_only_verification() {
        let test_router = protected_app(AuthRateLimiter::for_tests("redis://127.0.0.1:1/0".into()));

        for (method, path, expected_status) in [
            (Method::GET, "/api/v1/auth/verify-email", StatusCode::OK),
            (Method::POST, "/api/v1/auth/verify-email", StatusCode::OK),
            (
                Method::POST,
                "/api/v1/auth/register",
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                Method::POST,
                "/api/v1/auth/resend-verification",
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        ] {
            let response = test_router
                .clone()
                .oneshot(protected_request(
                    method,
                    path,
                    "192.0.2.1:45000".parse().unwrap(),
                    "198.51.100.1",
                ))
                .await
                .unwrap();

            assert_eq!(response.status(), expected_status);
        }
    }

    #[tokio::test]
    async fn malformed_trusted_proxy_address_is_server_error() {
        let peer_address: SocketAddr = "172.30.250.2:45000".parse().unwrap();
        let test_router = protected_app(
            AuthRateLimiter::for_tests("redis://127.0.0.1:1/0".into()).with_ip_policy_for_tests(
                vec![peer_address.ip()],
                crate::protection::rate_limit::BucketPolicy::new(1, 3600).unwrap(),
            ),
        );
        let response = test_router
            .oneshot(protected_request(
                Method::POST,
                "/api/v1/auth/register",
                peer_address,
                "invalid",
            ))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[tokio::test]
    #[ignore = "requires Redis; executed in CI with --include-ignored"]
    async fn trusted_proxy_isolates_clients_and_untrusted_proxy_shares_budget() {
        let _ = dotenvy::dotenv();
        let redis_url = std::env::var("REDIS_URL").expect("Redis test requires REDIS_URL");
        for proxy_is_trusted in [true, false] {
            let peer_address: SocketAddr = format!(
                "[2001:db8::{:x}:{:x}]:45000",
                rand::random::<u16>(),
                rand::random::<u16>()
            )
            .parse()
            .unwrap();
            let first_client_ip = format!(
                "2001:db8:1::{:x}:{:x}",
                rand::random::<u16>(),
                rand::random::<u16>()
            );
            let second_client_ip = format!(
                "2001:db8:2::{:x}:{:x}",
                rand::random::<u16>(),
                rand::random::<u16>()
            );
            let test_router = protected_app(
                AuthRateLimiter::for_tests(redis_url.clone()).with_ip_policy_for_tests(
                    if proxy_is_trusted {
                        vec![peer_address.ip()]
                    } else {
                        vec![]
                    },
                    crate::protection::rate_limit::BucketPolicy::new(1, 3600).unwrap(),
                ),
            );
            for (client_ip_header, expected_status) in [
                (first_client_ip.as_str(), StatusCode::ACCEPTED),
                (first_client_ip.as_str(), StatusCode::TOO_MANY_REQUESTS),
                (
                    second_client_ip.as_str(),
                    if proxy_is_trusted {
                        StatusCode::ACCEPTED
                    } else {
                        StatusCode::TOO_MANY_REQUESTS
                    },
                ),
            ] {
                let response = test_router
                    .clone()
                    .oneshot(protected_request(
                        Method::POST,
                        "/api/v1/auth/resend-verification",
                        peer_address,
                        client_ip_header,
                    ))
                    .await
                    .unwrap();

                assert_eq!(response.status(), expected_status);
            }
        }
    }

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
        let response = protection_error_response(true, ApiError::RateLimitUnavailable);

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            response.headers()["content-type"],
            "text/html; charset=utf-8"
        );
        assert!(!response.headers().contains_key("retry-after"));
    }

    #[tokio::test]
    async fn head_bypasses_client_identity_and_redis() {
        let rate_limiter = Arc::new(AuthRateLimiter::for_tests("redis://127.0.0.1:1/0".into()));

        let test_router = Router::new()
            .route(
                "/api/v1/auth/verify-email",
                get(|| async { StatusCode::OK }).head(|| async { StatusCode::METHOD_NOT_ALLOWED }),
            )
            .route_layer(middleware::from_fn_with_state(
                rate_limiter,
                limit_auth_requests,
            ));

        let response = test_router
            .oneshot(
                Request::builder()
                    .method(Method::HEAD)
                    .uri("/api/v1/auth/verify-email")
                    .body(axum::body::Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
