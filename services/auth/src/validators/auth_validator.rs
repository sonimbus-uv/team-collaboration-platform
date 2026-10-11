#[derive(PartialEq, Eq)]
pub struct ValidRegisterInput {
    pub email: String,
    pub password: String,
}

#[derive(Debug, PartialEq, Eq)]
pub enum RegisterInputValidationError {
    InvalidEmailFormat,
    PasswordTooShort,
    PasswordTooLong,
    InvalidPasswordFormat,
}

pub fn validate_register_input(
    email: String,
    password: String,
) -> Result<ValidRegisterInput, RegisterInputValidationError> {
    let email = email.trim().to_lowercase();

    if !is_valid_email_format(&email) {
        return Err(RegisterInputValidationError::InvalidEmailFormat);
    }

    let password_length = password.chars().count();

    if password_length < 8 {
        return Err(RegisterInputValidationError::PasswordTooShort);
    }

    if password_length > 128 {
        return Err(RegisterInputValidationError::PasswordTooLong);
    }

    if !is_valid_password_format(&password) {
        return Err(RegisterInputValidationError::InvalidPasswordFormat);
    }

    Ok(ValidRegisterInput { email, password })
}

pub fn is_valid_email_format(email: &str) -> bool {
    if email.len() > 254 || email.chars().any(char::is_whitespace) {
        return false;
    }

    if email.matches('@').count() != 1 {
        return false;
    }

    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };

    !local.is_empty()
        && local.len() <= 64
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && local
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b".!#$%&'*+-/=?^_`{|}~".contains(&b))
        && domain.contains('.')
        && domain.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
        })
}

impl std::fmt::Debug for ValidRegisterInput {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ValidRegisterInput")
            .field("email", &self.email)
            .field("password", &"[REDACTED]")
            .finish()
    }
}

fn is_valid_password_format(password: &str) -> bool {
    let has_uppercase = password.chars().any(|c| c.is_uppercase());
    let has_lowercase = password.chars().any(|c| c.is_lowercase());
    let has_digit = password.chars().any(|c| c.is_ascii_digit());
    let has_special_char = password.chars().any(|c| !c.is_alphanumeric());

    has_uppercase && has_lowercase && has_digit && has_special_char
}

#[cfg(test)]
mod tests {
    use super::{validate_register_input, RegisterInputValidationError};

    #[test]
    fn rejects_malformed_addresses() {
        for email in [
            "a@b..com",
            "a@-b.com",
            "a@b-.com",
            ".a@b.com",
            "a..b@b.com",
            "a@b_com.com",
            "a\u{0000}@b.com",
        ] {
            assert!(!super::is_valid_email_format(email), "{email:?}");
        }
        assert!(super::is_valid_email_format("name+tag@sub.example.com"));
    }

    #[test]
    fn debug_redacts_password() {
        let input = validate_register_input("a@b.com".into(), "ValidPass123!".into()).unwrap();
        assert!(!format!("{input:?}").contains("ValidPass123!"));
    }

    #[test]
    fn accepts_valid_register_input() {
        let input =
            validate_register_input("test@example.com".to_string(), "ValidPass123!".to_string())
                .expect("Expected valid input to pass validation");

        assert_eq!(input.email, "test@example.com");
        assert_eq!(input.password, "ValidPass123!");
    }

    #[test]
    fn normalizes_email() {
        let input = validate_register_input(
            " TEST@Example.COM ".to_string(),
            "ValidPass123!".to_string(),
        )
        .expect("Expected valid input to pass validation");

        assert_eq!(input.email, "test@example.com");
    }

    #[test]
    fn rejects_invalid_email_format() {
        let error =
            validate_register_input("invalid-email".to_string(), "ValidPass123!".to_string())
                .expect_err("Expected invalid email format to fail validation");

        assert_eq!(error, RegisterInputValidationError::InvalidEmailFormat);
    }

    #[test]
    fn rejects_short_password() {
        let error = validate_register_input("test@example.com".to_string(), "Weak1!".to_string())
            .expect_err("Expected short password to fail validation");

        assert_eq!(error, RegisterInputValidationError::PasswordTooShort);
    }

    #[test]
    fn rejects_long_password() {
        let long_password = format!("{}a1!", "A".repeat(126));

        let error = validate_register_input("test@example.com".to_string(), long_password)
            .expect_err("Expected long password to fail validation");

        assert_eq!(error, RegisterInputValidationError::PasswordTooLong);
    }

    #[test]
    fn rejects_invalid_password_format() {
        let error = validate_register_input(
            "test@example.com".to_string(),
            "invalidpassword".to_string(),
        )
        .expect_err("Expected invalid password format to fail validation");

        assert_eq!(error, RegisterInputValidationError::InvalidPasswordFormat);
    }
}
