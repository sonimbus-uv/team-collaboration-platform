use std::sync::Arc;

use auth_service::tokens::issuer::AccessTokenIssuer;
use axum::{extract::State, http::header, response::IntoResponse, routing::get, Json, Router};

const JWKS_CACHE_POLICY: &str = "public, max-age=300, must-revalidate";

/// Public key discovery must remain outside authentication and recipient budgets.
pub(super) fn router<S: Clone + Send + Sync + 'static>(
    token_issuer: Arc<AccessTokenIssuer>,
) -> Router<S> {
    Router::new()
        .route("/.well-known/jwks.json", get(show_public_keys))
        .with_state(token_issuer)
}

async fn show_public_keys(State(token_issuer): State<Arc<AccessTokenIssuer>>) -> impl IntoResponse {
    (
        [
            (header::CACHE_CONTROL, JWKS_CACHE_POLICY),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        Json(token_issuer.public_keys().clone()),
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
        let token_issuer = crate::tests::token_support::issuer();
        let test_router: Router = router(token_issuer.clone());

        for method in ["GET", "HEAD"] {
            let response = test_router
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
            assert_eq!(response.headers()[header::CACHE_CONTROL], JWKS_CACHE_POLICY);
            assert_eq!(response.headers()[header::CONTENT_TYPE], "application/json");

            let response_body = to_bytes(response.into_body(), 65_536).await.unwrap();
            if method == "HEAD" {
                assert!(response_body.is_empty());
                continue;
            }

            let jwks_document: serde_json::Value = serde_json::from_slice(&response_body).unwrap();
            assert_eq!(
                jwks_document,
                serde_json::to_value(token_issuer.public_keys()).unwrap()
            );

            for public_key in jwks_document["keys"].as_array().unwrap() {
                assert_eq!(public_key.as_object().unwrap().len(), 6);
                assert!(public_key.get("d").is_none());
                assert_eq!(public_key["alg"], "EdDSA");
            }
        }

        let response = test_router
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
