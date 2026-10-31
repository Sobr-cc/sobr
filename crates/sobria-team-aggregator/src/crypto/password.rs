//! Password hashing + verification (Argon2id PHC).
//!
//! Follows the exact pattern of `sobria-app::pairing`: default parameters
//! of `argon2 = 0.5` (OWASP: m=19456, t=2, p=1), salt from the OS RNG,
//! hash returned as a self-contained PHC string. No dependency on
//! OpenSSL.
//!
//! See <https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html#argon2id>.

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

use crate::error::{AggregatorError, AggregatorResult};

/// Minimum length of a user or admin password.
pub const MIN_PASSWORD_LEN: usize = 8;

/// Argon2id hash (PHC string) of a password.
pub fn hash_password(password: &str) -> AggregatorResult<String> {
    let salt = SaltString::generate(&mut OsRng);
    let phc = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|e| AggregatorError::Crypto(format!("argon2 hash: {e}")))?
        .to_string();
    Ok(phc)
}

/// `true` if `candidate` matches `stored_phc`.
///
/// Returns `false` if the PHC is malformed — no explicit error, so the
/// cause of an auth failure is not exposed.
#[must_use]
pub fn verify_password(stored_phc: &str, candidate: &str) -> bool {
    let Ok(parsed) = PasswordHash::new(stored_phc) else {
        return false;
    };
    Argon2::default()
        .verify_password(candidate.as_bytes(), &parsed)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_then_verify_roundtrip() {
        let phc = hash_password("hunter2-correct-horse").unwrap();
        assert!(phc.starts_with("$argon2id$"));
        assert!(verify_password(&phc, "hunter2-correct-horse"));
        assert!(!verify_password(&phc, "wrong-password"));
    }

    #[test]
    fn verify_rejects_malformed_phc() {
        assert!(!verify_password("not-a-phc", "anything"));
        assert!(!verify_password("", "anything"));
    }

    #[test]
    fn two_hashes_of_same_password_differ() {
        let a = hash_password("same").unwrap();
        let b = hash_password("same").unwrap();
        assert_ne!(a, b, "sels distincts => PHC distincts");
        assert!(verify_password(&a, "same"));
        assert!(verify_password(&b, "same"));
    }
}
