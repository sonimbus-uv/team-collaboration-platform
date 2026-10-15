use argon2::{
    password_hash::{
        rand_core::OsRng, Error as PasswordHashError, PasswordHash,
        PasswordHasher as Argon2PasswordHasherTrait, PasswordVerifier, SaltString,
    },
    Argon2,
};

#[derive(Debug)]
pub enum PasswordHashingError {
    HashFailed,
    InvalidHash,
}

pub trait PasswordHasher: Send + Sync {
    fn hash_password(&self, password: &str) -> Result<String, PasswordHashingError>;

    fn verify_password(
        &self,
        password: &str,
        password_hash: &str,
    ) -> Result<bool, PasswordHashingError>;
}

#[derive(Clone, Default)]
pub struct Argon2PasswordHasher;

impl PasswordHasher for Argon2PasswordHasher {
    fn hash_password(&self, password: &str) -> Result<String, PasswordHashingError> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|_| PasswordHashingError::HashFailed)
    }

    fn verify_password(
        &self,
        password: &str,
        password_hash: &str,
    ) -> Result<bool, PasswordHashingError> {
        let parsed_hash =
            PasswordHash::new(password_hash).map_err(|_| PasswordHashingError::InvalidHash)?;

        let argon2 = Argon2::default();

        match argon2.verify_password(password.as_bytes(), &parsed_hash) {
            Ok(()) => Ok(true),
            Err(PasswordHashError::Password) => Ok(false),
            Err(_) => Err(PasswordHashingError::InvalidHash),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn hashes_use_distinct_salts_and_verify_passwords() {
        let hasher = Argon2PasswordHasher;
        let first = hasher.hash_password("ValidPass123!").unwrap();
        let second = hasher.hash_password("ValidPass123!").unwrap();
        assert_ne!(first, second);
        assert!(first.starts_with("$argon2id$"));
        assert!(hasher.verify_password("ValidPass123!", &first).unwrap());
        assert!(!hasher.verify_password("WrongPass123!", &first).unwrap());
        assert!(matches!(
            hasher.verify_password("password", "broken"),
            Err(PasswordHashingError::InvalidHash)
        ));
    }
}
