pub mod config;
pub mod credentials;
pub mod limits;
pub mod server;

#[cfg(test)]
#[path = "../tests/internal_api.rs"]
mod tests;

use auth_service::{
    session_check_service::{SessionCheckError, SessionCheckService},
    session_check_types::{SessionCheckBatchRequest, SessionCheckRequest, MAX_SESSION_CHECKS},
};
use axum::{
    extract::{rejection::JsonRejection, ConnectInfo, DefaultBodyLimit, State},
    http::{header, HeaderValue, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::post,
    Extension, Json, Router,
};
use serde::Serialize;
use sqlx::{types::chrono::Utc, PgPool};
use std::{
    net::{IpAddr, SocketAddr},
    sync::Arc,
};
use tokio::{sync::Semaphore, time::timeout};

use config::InternalConfig;
use credentials::{Identity, ServiceCredentials};
use limits::{AdmissionError, InternalLimits};

pub struct InternalApi {
    pub config: InternalConfig,
    pub credentials: ServiceCredentials,
    pub limits: Arc<InternalLimits>,
    service: SessionCheckService,
    local_requests: Arc<Semaphore>,
}

impl InternalApi {
    pub fn new(
        config: InternalConfig,
        credentials: ServiceCredentials,
        redis_url: String,
        pool: PgPool,
    ) -> anyhow::Result<Arc<Self>> {
        let limits = InternalLimits::new(config.clone(), redis_url)?;
        Ok(Arc::new(Self {
            local_requests: Arc::new(Semaphore::new(config.local_limit)),
            config,
            credentials,
            limits,
            service: SessionCheckService::new(pool),
        }))
    }

    pub fn router(self: &Arc<Self>) -> Router {
        Router::new()
            .route("/internal/v1/sessions/check", post(check))
            .route("/internal/v1/sessions/check-batch", post(check_batch))
            .layer(DefaultBodyLimit::max(self.config.body_limit))
            .layer(middleware::from_fn_with_state(self.clone(), protect))
            .with_state(self.clone())
    }

    pub fn allows(&self, ip: IpAddr) -> bool {
        let ip = match ip {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(ip),
            _ => ip,
        };
        self.config
            .allowed_peers
            .iter()
            .any(|net| net.contains(&ip))
    }
}

async fn protect(
    State(api): State<Arc<InternalApi>>,
    mut request: axum::extract::Request,
    next: Next,
) -> Response {
    let started = tokio::time::Instant::now();
    let Ok(_permit) = api.local_requests.clone().try_acquire_owned() else {
        return problem(StatusCode::TOO_MANY_REQUESTS, Some(1));
    };
    let result = timeout(api.config.request_timeout, async {
        let Some(ConnectInfo(peer)) = request
            .extensions()
            .get::<ConnectInfo<SocketAddr>>()
            .copied()
        else {
            return problem(StatusCode::SERVICE_UNAVAILABLE, None);
        };
        if !api.allows(peer.ip()) {
            return problem(StatusCode::FORBIDDEN, None);
        }
        // IPv4-mapped socket addresses use the same budget as IPv4 addresses.
        let ip = match peer.ip() {
            IpAddr::V6(v6) => v6.to_ipv4_mapped().map(IpAddr::V4).unwrap_or(peer.ip()),
            ip => ip,
        };
        if let Err(error) = api.limits.preauthenticate(ip).await {
            return admission_error(error);
        }
        let Some((identity, allowed)) = api.credentials.authenticate(request.headers(), Utc::now())
        else {
            return problem(StatusCode::UNAUTHORIZED, None);
        };
        if !allowed {
            return problem(StatusCode::FORBIDDEN, None);
        }
        request.headers_mut().remove(header::AUTHORIZATION);
        if request.uri().query().is_some() {
            return problem(StatusCode::BAD_REQUEST, None);
        }
        request.extensions_mut().insert(identity);
        next.run(request).await
    })
    .await
    .unwrap_or_else(|_| problem(StatusCode::SERVICE_UNAVAILABLE, None));
    if started.elapsed() >= api.config.request_timeout {
        problem(StatusCode::SERVICE_UNAVAILABLE, None)
    } else {
        no_store(result)
    }
}

async fn check(
    State(api): State<Arc<InternalApi>>,
    Extension(identity): Extension<Identity>,
    request: Result<Json<SessionCheckRequest>, JsonRejection>,
) -> Response {
    let request = match request {
        Ok(Json(value)) => value,
        Err(error) => return json_error(error),
    };
    evaluate(&api, identity, 1, api.service.check(request)).await
}

async fn check_batch(
    State(api): State<Arc<InternalApi>>,
    Extension(identity): Extension<Identity>,
    request: Result<Json<SessionCheckBatchRequest>, JsonRejection>,
) -> Response {
    let request = match request {
        Ok(Json(value)) => value,
        Err(error) => return json_error(error),
    };
    let cost = request.sessions.len();
    if !(1..=MAX_SESSION_CHECKS).contains(&cost) {
        return problem(StatusCode::BAD_REQUEST, None);
    }
    evaluate(&api, identity, cost, api.service.check_batch(request)).await
}

async fn evaluate<T: Serialize>(
    api: &InternalApi,
    identity: Identity,
    cost: usize,
    query: impl std::future::Future<Output = Result<T, SessionCheckError>>,
) -> Response {
    let lease = match api.limits.reserve(identity, cost).await {
        Ok(lease) => lease,
        Err(error) => return admission_error(error),
    };
    let result = timeout(api.config.sql_timeout, query).await;
    if lease.release().await.is_err() {
        return problem(StatusCode::SERVICE_UNAVAILABLE, None);
    }
    match result {
        Ok(Ok(result)) => Json(result).into_response(),
        Ok(Err(SessionCheckError::InvalidInput(_))) => problem(StatusCode::BAD_REQUEST, None),
        // No error is logged with Debug, converted into inactivity, or exposed to callers.
        _ => problem(StatusCode::SERVICE_UNAVAILABLE, None),
    }
}

fn json_error(error: JsonRejection) -> Response {
    problem(
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            StatusCode::PAYLOAD_TOO_LARGE
        } else {
            StatusCode::BAD_REQUEST
        },
        None,
    )
}

fn admission_error(error: AdmissionError) -> Response {
    match error {
        AdmissionError::Limited(seconds) => problem(StatusCode::TOO_MANY_REQUESTS, Some(seconds)),
        AdmissionError::Unavailable => problem(StatusCode::SERVICE_UNAVAILABLE, None),
    }
}

pub(super) fn no_store(mut response: Response) -> Response {
    response
        .headers_mut()
        .insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
        .headers_mut()
        .insert(header::PRAGMA, HeaderValue::from_static("no-cache"));
    response.headers_mut().insert(
        header::X_CONTENT_TYPE_OPTIONS,
        HeaderValue::from_static("nosniff"),
    );
    response
}

pub(super) fn problem(status: StatusCode, retry: Option<u64>) -> Response {
    let kind = match status {
        StatusCode::BAD_REQUEST => "malformed-request",
        StatusCode::UNAUTHORIZED => "invalid-service-credentials",
        StatusCode::FORBIDDEN => "service-forbidden",
        StatusCode::PAYLOAD_TOO_LARGE => "request-too-large",
        StatusCode::TOO_MANY_REQUESTS => "service-rate-limited",
        _ => "session-check-unavailable",
    };
    let mut response = no_store((status, Json(serde_json::json!({
        "type":format!("/problems/{kind}"), "title":status.canonical_reason().unwrap_or("Error"),
        "status":status.as_u16(), "detail":"The session check could not be completed."
    }))).into_response());
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("application/problem+json"),
    );
    if status == StatusCode::UNAUTHORIZED {
        response
            .headers_mut()
            .insert(header::WWW_AUTHENTICATE, HeaderValue::from_static("Bearer"));
    }
    if let Some(seconds) = retry {
        response.headers_mut().insert(
            header::RETRY_AFTER,
            HeaderValue::from_str(&seconds.max(1).to_string()).expect("integer header"),
        );
    }
    response
}
