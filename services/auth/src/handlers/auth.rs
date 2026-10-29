use crate::{
    dto::auth::{
        RegisterRequest, RegisterResponse, RegistrationResponse, ResendVerificationRequest,
        VerifyEmailRequest,
    },
    email::verification_page::{verification_legacy_page, verification_page_response},
    error::ApiError,
    repositories::accounts_repository::CreateAccountError,
    services::{account_service::RegisterError, email_verification_service::VerificationError},
    state::AppState,
};
use axum::{
    extract::{rejection::JsonRejection, State},
    http::StatusCode,
    response::Response,
    Json,
};

fn input<T>(request: Result<Json<T>, JsonRejection>) -> Result<T, ApiError> {
    request.map(|Json(value)| value).map_err(|error| {
        if error.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::RequestTooLarge
        } else {
            ApiError::Validation("Invalid registration request".into())
        }
    })
}

fn response(
    attempt: crate::repositories::registration_attempts_repository::RegistrationAttempt,
    sent: bool,
) -> RegistrationResponse {
    RegistrationResponse {
        registration_attempt_id: attempt.id,
        expires_at: attempt
            .expires_at
            .format("%Y-%m-%dT%H:%M:%S%.fZ")
            .to_string(),
        verification_email_sent: sent,
    }
}

pub async fn register(
    State(state): State<AppState>,
    request: Result<Json<RegisterRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<RegistrationResponse>), ApiError> {
    let request = input(request)?;
    let (attempt, sent) = state.auth_service.register(request.email, request.password, state.auth_rate_limiter.as_ref()).await.map_err(|error| match error {
        RegisterError::InvalidInput(error) => ApiError::Validation(match error {
            crate::validators::auth_validator::RegisterInputValidationError::InvalidEmailFormat => "Invalid email format",
            crate::validators::auth_validator::RegisterInputValidationError::PasswordTooShort => "Password is too short",
            crate::validators::auth_validator::RegisterInputValidationError::PasswordTooLong => "Password is too long",
            crate::validators::auth_validator::RegisterInputValidationError::InvalidPasswordFormat => "Invalid password format",
        }.into()),
        RegisterError::CreateAccount(CreateAccountError::EmailAlreadyExists) => ApiError::Conflict("Email already registered".into()),
        RegisterError::CreateAccount(CreateAccountError::Database(error)) => database_error(error),
        RegisterError::Verification(error) => map_verification_error(error),
        RegisterError::Busy => ApiError::Busy,
        RegisterError::PasswordHashing(error) => {
            tracing::error!(invalid_hash = matches!(error, crate::security::password_hasher::PasswordHashingError::InvalidHash), "Registration password processing failed");
            ApiError::InternalServerError("Password processing failed".into())
        },
        RegisterError::Worker(error) => {
            tracing::error!(panicked = error.is_panic(), "Registration password worker failed");
            ApiError::InternalServerError("Password processing failed".into())
        },
    })?;
    Ok((StatusCode::CREATED, Json(response(attempt, sent))))
}

pub async fn verify_email(
    State(state): State<AppState>,
    request: Result<Json<VerifyEmailRequest>, JsonRejection>,
) -> Result<Json<RegisterResponse>, ApiError> {
    let request = input(request)?;
    let account = state
        .email_verification_service
        .confirm(request.registration_attempt_id, &request.code)
        .await
        .map_err(map_verification_error)?;
    Ok(Json(RegisterResponse::from(account)))
}

pub async fn verify_email_page() -> Response {
    // Legacy URLs are intentionally ignored, including any token/query parameters.
    verification_page_response(StatusCode::GONE, verification_legacy_page())
}

pub async fn resend_verification(
    State(state): State<AppState>,
    request: Result<Json<ResendVerificationRequest>, JsonRejection>,
) -> Result<(StatusCode, Json<RegistrationResponse>), ApiError> {
    let request = input(request)?;
    let (attempt, sent) = state
        .email_verification_service
        .resend(
            request.registration_attempt_id,
            state.auth_rate_limiter.as_ref(),
        )
        .await
        .map_err(map_verification_error)?;
    Ok((StatusCode::ACCEPTED, Json(response(attempt, sent))))
}

fn map_verification_error(error: VerificationError) -> ApiError {
    match error {
        VerificationError::Invalid => {
            ApiError::Validation("Invalid or unavailable registration attempt or code".into())
        }
        VerificationError::Limited(retry_after_secs) => ApiError::RateLimited { retry_after_secs },
        VerificationError::AlreadyRegistered => {
            ApiError::Conflict("Email already registered".into())
        }
        VerificationError::Database(error) => database_error(error),
        VerificationError::Unavailable => ApiError::RateLimitUnavailable,
    }
}

fn database_error(error: sqlx::Error) -> ApiError {
    // Never format SQL errors: PostgreSQL details can contain the row's code MAC/hash.
    tracing::warn!(
        pool_closed = matches!(error, sqlx::Error::PoolClosed),
        "Registration database operation unavailable"
    );
    ApiError::RateLimitUnavailable
}

pub async fn registration_headers(
    request: axum::extract::Request,
    next: axum::middleware::Next,
) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert("cache-control", "no-store".parse().unwrap());
    response
        .headers_mut()
        .insert("pragma", "no-cache".parse().unwrap());
    response
        .headers_mut()
        .insert("x-content-type-options", "nosniff".parse().unwrap());
    response
}
