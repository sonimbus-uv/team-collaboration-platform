use std::{net::SocketAddr, sync::Arc};

mod jwks;

use crate::{
    email::verification_page::{verification_page_response, verification_unavailable_page},
    error::ApiError,
    handlers::auth,
    protection::auth_rate_limiter::AuthRateLimiter,
    protection::client_ip::resolve_client_ip,
    protection::rate_limit::{AuthOperation, RateLimitDecision},
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
    let public_keys = jwks::router(state.token_issuer.clone());
    let limiter = state.auth_rate_limiter.clone();

    let session_routes = Router::new()
        .route("/api/v1/auth/login", post(crate::handlers::sessions::login))
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
            limiter.clone(),
            limit_auth_requests,
        ));

    let auth_routes = Router::new()
        .route("/auth/register", post(auth::register))
        .route(
            "/auth/verify-email",
            get(auth::verify_email_page)
                .head(|| async {
                    verification_page_response(StatusCode::METHOD_NOT_ALLOWED, String::new())
                })
                .post(auth::verify_email),
        )
        .route("/auth/resend-verification", post(auth::resend_verification))
        .route_layer(middleware::from_fn_with_state(limiter, limit_auth_requests));

    Router::new()
        .route("/health", get(health))
        .route("/health/db", get(health_db))
        .route("/health/redis", get(health_redis))
        .merge(auth_routes)
        .merge(session_routes)
        .merge(public_keys)
        .with_state(state)
}

async fn limit_auth_requests(
    State(limiter): State<Arc<AuthRateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    let Some(operation) = AuthOperation::from_request(request.method(), request.uri().path())
    else {
        return next.run(request).await;
    };

    let respond = |browser, error| {
        if operation.is_session() {
            crate::handlers::sessions::protection(error)
        } else {
            protection_error_response(browser, error)
        }
    };

    let browser_verification =
        request.method() == Method::GET && operation == AuthOperation::Verify;

    let Some(peer) = request
        .extensions()
        .get::<ConnectInfo<SocketAddr>>()
        .map(|info| info.0)
    else {
        tracing::error!("Connection information missing for protected Auth request");

        return respond(browser_verification, ApiError::RateLimitUnavailable);
    };

    let client_ip = match resolve_client_ip(peer, request.headers(), limiter.trusted_proxy_ips()) {
        Ok(ip) => ip,
        Err(_) => {
            return respond(
                browser_verification,
                ApiError::InternalServerError(
                    "Missing or invalid client address from trusted proxy".into(),
                ),
            );
        }
    };

    match limiter.check_ip(operation, client_ip).await {
        Ok(RateLimitDecision::Allowed) => next.run(request).await,
        Ok(RateLimitDecision::Limited { retry_after_secs }) => respond(
            browser_verification,
            ApiError::RateLimited { retry_after_secs },
        ),
        Err(error) if operation == AuthOperation::Verify => {
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

            respond(browser_verification, ApiError::RateLimitUnavailable)
        }
    }
}

fn protection_error_response(browser_verification: bool, error: ApiError) -> Response {
    let api_response = error.into_response();

    if !browser_verification {
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

    fn protected_app(limiter: AuthRateLimiter) -> Router {
        Router::new()
            .route("/auth/register", post(|| async { StatusCode::CREATED }))
            .route(
                "/auth/resend-verification",
                post(|| async { StatusCode::ACCEPTED }),
            )
            .route(
                "/auth/verify-email",
                get(|| async { StatusCode::OK }).post(|| async { StatusCode::OK }),
            )
            .route_layer(middleware::from_fn_with_state(
                Arc::new(limiter),
                limit_auth_requests,
            ))
    }

    fn protected_request(method: Method, path: &str, peer: SocketAddr, client: &str) -> Request {
        Request::builder()
            .method(method)
            .uri(path)
            .extension(ConnectInfo(peer))
            .header("x-real-ip", client)
            .body(axum::body::Body::empty())
            .unwrap()
    }

    #[tokio::test]
    async fn redis_failure_allows_only_verification() {
        let app = protected_app(AuthRateLimiter::for_tests("redis://127.0.0.1:1/0".into()));
        for (method, path, expected) in [
            (Method::GET, "/auth/verify-email", StatusCode::OK),
            (Method::POST, "/auth/verify-email", StatusCode::OK),
            (
                Method::POST,
                "/auth/register",
                StatusCode::SERVICE_UNAVAILABLE,
            ),
            (
                Method::POST,
                "/auth/resend-verification",
                StatusCode::SERVICE_UNAVAILABLE,
            ),
        ] {
            let response = app
                .clone()
                .oneshot(protected_request(
                    method,
                    path,
                    "192.0.2.1:45000".parse().unwrap(),
                    "198.51.100.1",
                ))
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
        }
    }

    #[tokio::test]
    async fn malformed_trusted_proxy_address_is_server_error() {
        let peer: SocketAddr = "172.30.250.2:45000".parse().unwrap();
        let app = protected_app(
            AuthRateLimiter::for_tests("redis://127.0.0.1:1/0".into()).with_ip_policy_for_tests(
                vec![peer.ip()],
                crate::protection::rate_limit::BucketPolicy::new(1, 3600).unwrap(),
            ),
        );
        let response = app
            .oneshot(protected_request(
                Method::POST,
                "/auth/register",
                peer,
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
        for trusted in [true, false] {
            // Unique peers/clients avoid sharing keys with other tests or prior runs.
            let peer: SocketAddr = format!(
                "[2001:db8::{:x}:{:x}]:45000",
                rand::random::<u16>(),
                rand::random::<u16>()
            )
            .parse()
            .unwrap();
            let first = format!(
                "2001:db8:1::{:x}:{:x}",
                rand::random::<u16>(),
                rand::random::<u16>()
            );
            let second = format!(
                "2001:db8:2::{:x}:{:x}",
                rand::random::<u16>(),
                rand::random::<u16>()
            );
            let app = protected_app(
                AuthRateLimiter::for_tests(redis_url.clone()).with_ip_policy_for_tests(
                    if trusted { vec![peer.ip()] } else { vec![] },
                    crate::protection::rate_limit::BucketPolicy::new(1, 3600).unwrap(),
                ),
            );
            for (client, expected) in [
                (first.as_str(), StatusCode::ACCEPTED),
                (first.as_str(), StatusCode::TOO_MANY_REQUESTS),
                (
                    second.as_str(),
                    if trusted {
                        StatusCode::ACCEPTED
                    } else {
                        StatusCode::TOO_MANY_REQUESTS
                    },
                ),
            ] {
                let response = app
                    .clone()
                    .oneshot(protected_request(
                        Method::POST,
                        "/auth/resend-verification",
                        peer,
                        client,
                    ))
                    .await
                    .unwrap();
                assert_eq!(response.status(), expected);
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
        let limiter = Arc::new(AuthRateLimiter::for_tests("redis://127.0.0.1:1/0".into()));

        let app = Router::new()
            .route(
                "/auth/verify-email",
                get(|| async { StatusCode::OK }).head(|| async { StatusCode::METHOD_NOT_ALLOWED }),
            )
            .route_layer(middleware::from_fn_with_state(limiter, limit_auth_requests));

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
