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
        let parsed_password_hash =
            PasswordHash::new(password_hash).map_err(|_| PasswordHashingError::InvalidHash)?;
        if parsed_password_hash.salt.is_none() || parsed_password_hash.hash.is_none() {
            return Err(PasswordHashingError::InvalidHash);
        }
        let argon2 = Argon2::default();

        match argon2.verify_password(password.as_bytes(), &parsed_password_hash) {
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
        let first_password_hash = hasher.hash_password("ValidPass123!").unwrap();
        let second_password_hash = hasher.hash_password("ValidPass123!").unwrap();

        assert_ne!(first_password_hash, second_password_hash);
        assert!(first_password_hash.starts_with("$argon2id$"));
        assert!(hasher
            .verify_password("ValidPass123!", &first_password_hash)
            .unwrap());
        assert!(!hasher
            .verify_password("WrongPass123!", &first_password_hash)
            .unwrap());
        assert!(matches!(
            hasher.verify_password("password", "broken"),
            Err(PasswordHashingError::InvalidHash)
        ));
    }
    #[test]
    fn malformed_password_hashes_are_errors_not_password_mismatches() {
        let hasher = Argon2PasswordHasher;
        for password_hash in [
            "",
            "not-a-phc-hash",
            "$argon2id$",
            "$argon2id$v=invalid$",
            "$unsupported$v=19$m=8,t=1,p=1$c2FsdHNhbHQ$YWJjZA",
        ] {
            assert!(matches!(
                hasher.verify_password("test-password", password_hash),
                Err(PasswordHashingError::InvalidHash)
            ));
        }
        let valid_hash = hasher.hash_password("test-password").unwrap();
        let unsupported_version = valid_hash.replace("$v=19$", "$v=999$");
        let missing_digest = valid_hash.rsplit_once('$').unwrap().0;
        for (case, password_hash) in [
            ("unsupported version", unsupported_version.as_str()),
            ("missing digest", missing_digest),
        ] {
            let result = hasher.verify_password("test-password", password_hash);
            assert!(
                matches!(result, Err(PasswordHashingError::InvalidHash)),
                "{case}: {result:?}"
            );
        }
    }

    #[test]
    fn password_whitespace_and_embedded_null_bytes_are_preserved() {
        let hasher = Argon2PasswordHasher;
        for (password, altered_password) in [
            (" Password123! ", "Password123!"),
            ("Password123!\0suffix", "Password123!"),
        ] {
            let password_hash = hasher.hash_password(password).unwrap();
            assert!(hasher.verify_password(password, &password_hash).unwrap());
            assert!(!hasher
                .verify_password(altered_password, &password_hash)
                .unwrap());
        }
    }
}
