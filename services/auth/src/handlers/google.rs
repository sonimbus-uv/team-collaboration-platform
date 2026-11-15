use std::collections::HashMap;

use base64::Engine;

use axum::{
    extract::{rejection::JsonRejection, RawQuery, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use reqwest::Url;

use super::{
    authentication::{access_error, authenticated_actor},
    response::{json_error, problem, protected, rate_limited},
};
use crate::{
    dto::google::{CreateGoogleAttempt, ExchangeGoogleAttempt, LinkGoogleAttempt, Reauthenticate},
    protection::rate_limit::RateLimitDecision,
    services::google_auth_service::{hash, GoogleAuthError, GoogleCallback, GoogleExchangeResult},
    state::AppState,
};

const MAX_QUERY_BYTES: usize = 8192;
const MAX_COOKIE_HEADER_BYTES: usize = 16384;
const ATTEMPT_SECRET_HEADER: &str = "x-auth-attempt-secret";
const BROWSER_COOKIE_MAX_AGE_SECONDS: u64 = 600;

pub async fn create(
    State(state): State<AppState>,
    request: Result<Json<CreateGoogleAttempt>, JsonRejection>,
) -> Response {
    let Some(service) = &state.google_service else {
        return google_error_response(GoogleAuthError::Unavailable);
    };

    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => return json_error(error),
    };

    match service.create(request).await {
        Ok(attempt) => protected((StatusCode::CREATED, Json(attempt)).into_response()),
        Err(error) => google_error_response(error),
    }
}

pub async fn authorize(State(state): State<AppState>, RawQuery(query): RawQuery) -> Response {
    let Some(service) = &state.google_service else {
        return google_error_response(GoogleAuthError::Unavailable);
    };

    let query = match query_parameters(query.as_deref()) {
        Ok(query) => query,
        Err(error) => return google_error_response(error),
    };

    let Some(ticket) = query.get("ticket") else {
        return google_error_response(GoogleAuthError::Malformed);
    };

    match service.authorize(ticket).await {
        Ok(authorization) => {
            let response = redirect(&authorization.url);
            with_browser_cookie(
                response,
                &authorization.state,
                BROWSER_COOKIE_MAX_AGE_SECONDS,
            )
        }
        Err(error) => google_error_response(error),
    }
}

pub async fn callback(
    State(state): State<AppState>,
    RawQuery(query): RawQuery,
    headers: HeaderMap,
) -> Response {
    let Some(service) = &state.google_service else {
        return google_error_response(GoogleAuthError::Unavailable);
    };

    let query = match query_parameters(query.as_deref()) {
        Ok(query) => query,
        Err(error) => return google_error_response(error),
    };

    let Some(oauth_state) = query.get("state") else {
        return google_error_response(GoogleAuthError::InvalidProof);
    };
    if let Err(error) = validate_callback_parameters(&query) {
        return google_error_response(error);
    }
    let cookie = match browser_cookie(&headers, &cookie_name(oauth_state)) {
        Some(cookie) => cookie,
        None => return google_error_response(GoogleAuthError::InvalidProof),
    };

    let result = service
        .callback(GoogleCallback {
            state: oauth_state,
            cookie_state: &cookie,
            code: query.get("code").map(String::as_str),
            denied: query
                .get("error")
                .is_some_and(|error| error == "access_denied"),
        })
        .await;
    let response = match result {
        Ok(result) => redirect(result.redirect.as_str()),
        Err(error) => google_error_response(error),
    };
    with_browser_cookie(response, oauth_state, 0)
}

pub async fn exchange(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<ExchangeGoogleAttempt>, JsonRejection>,
) -> Response {
    let Some(service) = &state.google_service else {
        return google_error_response(GoogleAuthError::Unavailable);
    };

    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => return json_error(error),
    };

    let secret = match attempt_secret(&headers) {
        Ok(secret) => secret,
        Err(error) => return google_error_response(error),
    };

    match service.exchange(request, secret).await {
        Ok(GoogleExchangeResult::Tokens(tokens)) => protected(Json(tokens).into_response()),
        Ok(GoogleExchangeResult::Linked) => protected(StatusCode::NO_CONTENT.into_response()),
        Err(error) => google_error_response(error),
    }
}

pub async fn create_link(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<LinkGoogleAttempt>, JsonRejection>,
) -> Response {
    let Some(service) = &state.google_service else {
        return google_error_response(GoogleAuthError::Unavailable);
    };

    let actor = match authenticated_actor(&state, &headers).await {
        Ok(actor) => actor,
        Err(error) => return access_error(error),
    };

    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => return json_error(error),
    };

    match service.create_link(actor, request).await {
        Ok(attempt) => protected((StatusCode::CREATED, Json(attempt)).into_response()),
        Err(error) => google_error_response(error),
    }
}

pub async fn connections(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let Some(service) = &state.google_service else {
        return google_error_response(GoogleAuthError::Unavailable);
    };

    let actor = match authenticated_actor(&state, &headers).await {
        Ok(actor) => actor,
        Err(error) => return access_error(error),
    };

    match service.connections(actor).await {
        Ok(connections) => protected(Json(connections).into_response()),
        Err(error) => google_error_response(error),
    }
}

pub async fn reauthenticate(
    State(state): State<AppState>,
    headers: HeaderMap,
    request: Result<Json<Reauthenticate>, JsonRejection>,
) -> Response {
    if state.google_service.is_none() {
        return google_error_response(GoogleAuthError::Unavailable);
    }
    let actor = match authenticated_actor(&state, &headers).await {
        Ok(actor) => actor,
        Err(error) => return access_error(error),
    };

    let request = match request {
        Ok(Json(request)) => request,
        Err(error) => return json_error(error),
    };

    match state
        .auth_rate_limiter
        .check_reauthentication(actor.account_id)
        .await
    {
        Ok(RateLimitDecision::Allowed) => {}
        Ok(RateLimitDecision::Limited { retry_after_secs }) => {
            return rate_limited(retry_after_secs)
        }
        Err(_) => return google_error_response(GoogleAuthError::Unavailable),
    }

    match state
        .sessions()
        .reauthenticate(actor, request.password)
        .await
    {
        Ok(()) => protected(StatusCode::NO_CONTENT.into_response()),
        Err(error) => super::sessions::from_error(error),
    }
}

fn query_parameters(query: Option<&str>) -> Result<HashMap<String, String>, GoogleAuthError> {
    let query = query.ok_or(GoogleAuthError::Malformed)?;
    if query.len() > MAX_QUERY_BYTES {
        return Err(GoogleAuthError::Malformed);
    }
    let url = Url::parse(&format!("https://query.invalid/?{query}"))
        .map_err(|_| GoogleAuthError::Malformed)?;
    let mut parameters = HashMap::new();
    for (key, value) in url.query_pairs() {
        if parameters
            .insert(key.into_owned(), value.into_owned())
            .is_some()
        {
            return Err(GoogleAuthError::Malformed);
        }
    }
    Ok(parameters)
}

fn cookie_name(state: &str) -> String {
    format!(
        "__Host-auth-google-{}",
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hash(state))
    )
}

fn browser_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let mut matching_cookie = None;
    for cookie_header in headers.get_all(header::COOKIE) {
        let cookies = cookie_header.to_str().ok()?;
        if cookies.len() > MAX_COOKIE_HEADER_BYTES {
            return None;
        }
        for entry in cookies.split(';') {
            let Some((cookie_name, cookie_value)) = entry.trim().split_once('=') else {
                continue;
            };
            if cookie_name != name {
                continue;
            }
            if matching_cookie.is_some() {
                return None;
            }
            matching_cookie = Some(cookie_value.to_owned());
        }
    }
    matching_cookie
}

fn validate_callback_parameters(query: &HashMap<String, String>) -> Result<(), GoogleAuthError> {
    if query.contains_key("code") == query.contains_key("error") {
        return Err(GoogleAuthError::InvalidProof);
    }
    if let Some(issuer) = query.get("iss") {
        if !matches!(
            issuer.as_str(),
            "https://accounts.google.com" | "accounts.google.com"
        ) {
            return Err(GoogleAuthError::InvalidProof);
        }
    }
    Ok(())
}

fn attempt_secret(headers: &HeaderMap) -> Result<&str, GoogleAuthError> {
    let mut secrets = headers.get_all(ATTEMPT_SECRET_HEADER).iter();
    let secret = secrets
        .next()
        .and_then(|value| value.to_str().ok())
        .ok_or(GoogleAuthError::InvalidProof)?;
    if secrets.next().is_some() {
        return Err(GoogleAuthError::InvalidProof);
    }
    Ok(secret)
}

fn with_browser_cookie(mut response: Response, state: &str, max_age_seconds: u64) -> Response {
    let cookie_value = if max_age_seconds == 0 { "" } else { state };
    let cookie = format!(
        "{}={cookie_value}; Path=/; Secure; HttpOnly; SameSite=Lax; Max-Age={max_age_seconds}",
        cookie_name(state),
    );
    match cookie.parse() {
        Ok(value) => {
            response.headers_mut().insert(header::SET_COOKIE, value);
            response
        }
        Err(_) => google_error_response(GoogleAuthError::Unavailable),
    }
}

fn redirect(location: &str) -> Response {
    let Ok(location) = location.parse() else {
        return google_error_response(GoogleAuthError::Unavailable);
    };

    let mut response = protected(StatusCode::FOUND.into_response());
    response.headers_mut().insert(header::LOCATION, location);
    response.headers_mut().insert(
        header::REFERRER_POLICY,
        header::HeaderValue::from_static("no-referrer"),
    );
    response
}

fn google_error_response(error: GoogleAuthError) -> Response {
    let (status, kind) = match error {
        GoogleAuthError::Malformed => (StatusCode::BAD_REQUEST, "malformed-request"),
        GoogleAuthError::InvalidAccess => (StatusCode::UNAUTHORIZED, "invalid-access-token"),
        GoogleAuthError::ReauthenticationRequired => {
            (StatusCode::UNAUTHORIZED, "reauthentication-required")
        }
        GoogleAuthError::IdentityLinked => (StatusCode::CONFLICT, "identity-already-linked"),
        GoogleAuthError::ProviderLinked => (StatusCode::CONFLICT, "provider-already-linked"),
        GoogleAuthError::LinkConflict => (StatusCode::CONFLICT, "account-link-conflict"),
        GoogleAuthError::InvalidProof => (StatusCode::UNAUTHORIZED, "invalid-google-credentials"),
        GoogleAuthError::AccountBlocked => (StatusCode::FORBIDDEN, "account-not-allowed"),
        GoogleAuthError::LinkRequired => (StatusCode::CONFLICT, "account-link-required"),
        GoogleAuthError::Unavailable => (StatusCode::SERVICE_UNAVAILABLE, "auth-unavailable"),
        GoogleAuthError::CommitUncertain => (StatusCode::INTERNAL_SERVER_ERROR, "internal-error"),
        GoogleAuthError::Database(error) => return super::response::database_error(error),
        GoogleAuthError::Session(error) => return super::sessions::from_error(error),
    };

    problem(status, kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn security_parameters_and_browser_cookies_cannot_be_ambiguous() {
        for query in [
            "state=a&state=b",
            "code=a&code=b",
            "error=a&error=b",
            "state=a&st%61te=b",
        ] {
            assert!(query_parameters(Some(query)).is_err());
        }
        let mut headers = HeaderMap::new();
        headers.insert(header::COOKIE, "proof=a; proof=b".parse().unwrap());
        assert!(browser_cookie(&headers, "proof").is_none());
        headers.insert(header::COOKIE, "other=ignored; proof=a".parse().unwrap());
        assert_eq!(browser_cookie(&headers, "proof").as_deref(), Some("a"));
    }

    #[test]
    fn callback_requires_one_outcome_and_a_trusted_issuer() {
        for query in [
            "state=a",
            "state=a&code=b&error=access_denied",
            "state=a&code=b&iss=https://untrusted.invalid",
        ] {
            let parameters = query_parameters(Some(query))
                .unwrap_or_else(|_| panic!("Expected valid query syntax"));
            assert!(validate_callback_parameters(&parameters).is_err());
        }
        for query in [
            "state=a&code=b",
            "state=a&error=access_denied",
            "state=a&code=b&iss=https://accounts.google.com",
            "state=a&code=b&iss=accounts.google.com",
        ] {
            let parameters = query_parameters(Some(query))
                .unwrap_or_else(|_| panic!("Expected valid query syntax"));
            assert!(validate_callback_parameters(&parameters).is_ok());
        }
    }

    #[test]
    fn attempt_secret_rejects_missing_and_duplicate_headers() {
        let mut headers = HeaderMap::new();
        assert!(attempt_secret(&headers).is_err());
        headers.insert(ATTEMPT_SECRET_HEADER, "secret".parse().unwrap());
        assert!(matches!(attempt_secret(&headers), Ok("secret")));
        headers.append(ATTEMPT_SECRET_HEADER, "other".parse().unwrap());
        assert!(attempt_secret(&headers).is_err());
    }

    #[test]
    fn browser_cookie_rejects_duplicates_across_headers() {
        let mut headers = HeaderMap::new();
        headers.append(header::COOKIE, "proof=a".parse().unwrap());
        headers.append(header::COOKIE, "proof=b".parse().unwrap());
        assert!(browser_cookie(&headers, "proof").is_none());
    }
}
