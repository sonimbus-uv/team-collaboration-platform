use std::sync::Arc;

use auth_service::tokens::issuer::AccessTokenIssuer;
use axum::{extract::State, http::header, response::IntoResponse, routing::get, Json, Router};

const CACHE_POLICY: &str = "public, max-age=300, must-revalidate";

/// Mounted outside authentication/recipient budgets; immutable validated snapshot.
pub(super) fn router<S: Clone + Send + Sync + 'static>(
    issuer: Arc<AccessTokenIssuer>,
) -> Router<S> {
    Router::new()
        .route("/.well-known/jwks.json", get(show))
        .with_state(issuer)
}

async fn show(State(issuer): State<Arc<AccessTokenIssuer>>) -> impl IntoResponse {
    (
        [
            (header::CACHE_CONTROL, CACHE_POLICY),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        Json(issuer.public_keys().clone()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{to_bytes, Body},
        http::{Request, StatusCode},
    };
    use tower::ServiceExt;

    #[tokio::test]
    async fn public_get_and_head_work_without_auth_peer_database_or_redis() {
        let issuer = crate::token_test_support::issuer();
        let app: Router = router(issuer.clone());
        for method in ["GET", "HEAD"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri("/.well-known/jwks.json")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            assert_eq!(response.headers()[header::CACHE_CONTROL], CACHE_POLICY);
            assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");
            let body = to_bytes(response.into_body(), 65_536).await.unwrap();
            if method == "HEAD" {
                assert!(body.is_empty());
                continue;
            }
            let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
            assert_eq!(json, serde_json::to_value(issuer.public_keys()).unwrap());
            for key in json["keys"].as_array().unwrap() {
                assert_eq!(key.as_object().unwrap().len(), 6);
                assert!(key.get("d").is_none());
                assert_eq!(key["alg"], "EdDSA");
            }
        }
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/.well-known/jwks.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::METHOD_NOT_ALLOWED);
    }
}
