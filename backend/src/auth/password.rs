use argon2::password_hash::SaltString;
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use rand::rngs::OsRng;

use crate::error::{AppError, AppResult};

/// Hash a password using Argon2id
pub fn hash_password(password: &str) -> AppResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let argon2 = Argon2::default();
    let hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AppError::Internal(format!("failed to hash password: {e}")))?;
    Ok(hash.to_string())
}

/// Verify a password against a hash
pub fn verify_password(password: &str, hash: &str) -> AppResult<bool> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|e| AppError::Internal(format!("invalid password hash: {e}")))?;
    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_password_returns_valid_hash() {
        let password = "my_secure_password";
        let hash = hash_password(password).unwrap();

        // Argon2 hash should start with $argon2
        assert!(hash.starts_with("$argon2"), "hash should be Argon2 format");
        // Hash should be different from the original password
        assert_ne!(hash, password);
    }

    #[test]
    fn test_verify_password_correct() {
        let password = "my_secure_password";
        let hash = hash_password(password).unwrap();

        let result = verify_password(password, &hash).unwrap();
        assert!(result, "correct password should verify successfully");
    }

    #[test]
    fn test_verify_password_incorrect() {
        let password = "my_secure_password";
        let wrong_password = "wrong_password";
        let hash = hash_password(password).unwrap();

        let result = verify_password(wrong_password, &hash).unwrap();
        assert!(!result, "incorrect password should fail verification");
    }

    #[test]
    fn test_hash_is_unique_each_time() {
        let password = "my_secure_password";
        let hash1 = hash_password(password).unwrap();
        let hash2 = hash_password(password).unwrap();

        // Same password should produce different hashes due to random salt
        assert_ne!(hash1, hash2, "hashes should be unique due to salt");

        // But both should verify correctly
        assert!(verify_password(password, &hash1).unwrap());
        assert!(verify_password(password, &hash2).unwrap());
    }

    #[test]
    fn test_verify_invalid_hash_format() {
        let result = verify_password("password", "not_a_valid_hash");
        assert!(result.is_err(), "invalid hash format should return error");
    }

    #[test]
    fn test_empty_password() {
        let password = "";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("not_empty", &hash).unwrap());
    }
}
