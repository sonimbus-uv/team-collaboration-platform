use axum::{extract::State, http::StatusCode, Json};

use crate::{
    dto::auth::{RegisterRequest, RegisterResponse, ResendVerificationRequest, VerifyEmailRequest},
    error::ApiError,
    repositories::accounts_repository::CreateAccountError,
    security::password_hasher::PasswordHashingError,
    services::account_service::RegisterError,
    services::email_verification_service::VerificationError,
    state::AppState,
    validators::auth_validator::RegisterInputValidationError,
};

pub async fn register(
    State(state): State<AppState>,
    Json(request): Json<RegisterRequest>,
) -> Result<(StatusCode, Json<RegisterResponse>), ApiError> {
    let (account, sent) = state
        .auth_service
        .register(request.email, request.password)
        .await
        .map_err(map_register_error)?;

    let mut response = RegisterResponse::from(account);
    response.verification_email_sent = sent;
    Ok((StatusCode::CREATED, Json(response)))
}

fn map_register_error(error: RegisterError) -> ApiError {
    match error {
        RegisterError::InvalidInput(error) => map_register_validation_error(error),
        RegisterError::PasswordHashing(error) => map_password_hashing_error(error),
        RegisterError::CreateAccount(error) => map_create_account_error(error),
        RegisterError::Busy => ApiError::Busy,
        RegisterError::Worker(error) => {
            tracing::error!(?error, "password worker failed");
            ApiError::InternalServerError("Password worker failed".into())
        }
    }
}

fn map_register_validation_error(error: RegisterInputValidationError) -> ApiError {
    match error {
        RegisterInputValidationError::InvalidEmailFormat => {
            ApiError::Validation("Invalid email format".to_string())
        }
        RegisterInputValidationError::PasswordTooShort => {
            ApiError::Validation("Password is too short".to_string())
        }
        RegisterInputValidationError::PasswordTooLong => {
            ApiError::Validation("Password is too long".to_string())
        }
        RegisterInputValidationError::InvalidPasswordFormat => {
            ApiError::Validation("Invalid password format".to_string())
        }
    }
}

fn map_password_hashing_error(error: PasswordHashingError) -> ApiError {
    tracing::error!(?error, "password hashing failed");
    match error {
        PasswordHashingError::HashFailed => {
            ApiError::InternalServerError("Password hashing failed".to_string())
        }
        PasswordHashingError::InvalidHash => {
            ApiError::InternalServerError("Invalid password hash".to_string())
        }
    }
}

fn map_create_account_error(error: CreateAccountError) -> ApiError {
    match error {
        CreateAccountError::EmailAlreadyExists => {
            ApiError::Conflict("Email already exists".to_string())
        }
        CreateAccountError::Database(error) => ApiError::Database(error),
    }
}

pub async fn verify_email(
    State(state): State<AppState>,
    Json(request): Json<VerifyEmailRequest>,
) -> Result<Json<RegisterResponse>, ApiError> {
    let account = state
        .email_verification_service
        .confirm(&request.token)
        .await
        .map_err(map_verification_error)?;
    Ok(Json(RegisterResponse::from(account)))
}

pub async fn resend_verification(
    State(state): State<AppState>,
    Json(request): Json<ResendVerificationRequest>,
) -> Result<StatusCode, ApiError> {
    let email = request.email.trim().to_lowercase();
    if !crate::validators::auth_validator::is_valid_email_format(&email) {
        return Err(ApiError::Validation("Invalid email format".into()));
    }
    state
        .email_verification_service
        .resend(&email)
        .await
        .map_err(map_verification_error)?;
    Ok(StatusCode::ACCEPTED)
}

fn map_verification_error(error: VerificationError) -> ApiError {
    match error {
        VerificationError::InvalidToken => {
            ApiError::Validation("Invalid or expired verification token".into())
        }
        VerificationError::Database(error) => ApiError::Database(error),
        VerificationError::Delivery(error) => {
            tracing::error!(error = %error, "verification email delivery failed");
            ApiError::InternalServerError("Email delivery failed".into())
        }
    }
}
