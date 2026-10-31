//! Sobr.ia — browser extension ↔ desktop app pairing.
//!
//! Pure pairing logic with a **6-digit code**, TTL 5 min, single-use,
//! constant-time comparison. Independent of SQLite and Tauri: `AppState`
//! holds the pending code and `ExtensionStore` persists `device_pairings`.
//!
//! ## Security
//!
//! - **6-digit code**: 20 bits of entropy, OS RNG (`rand::thread_rng`).
//!   Enough for short-lived local use (TTL 5 min, single-use).
//! - **Shared secret**: 32 random bytes, hashed with **Argon2id** (default
//!   params of the `argon2 = 0.5` crate, set from the OWASP cheat
//!   sheet) for storage. The returned hash is a **PHC string** that
//!   holds the params + the salt + the base64 hash, and is
//!   self-contained for `verify_password`. Replaces the previous
//!   SHA-256 + manual salt implementation.
//! - **Code comparison**: constant-time to prevent timing attacks
//!   (even if the risk is low locally).
//!
//! ## Usage
//!
//! The flow of `bridge_server` and `logic::verify_pairing_code`:
//! ```ignore
//! // "Régénérer" in "Extension navigateur": a new code waits in AppState.
//! *state.pending_code.lock()? = Some(PendingCode::new());
//! // The bridge forwards the code typed in the extension:
//! verify_code(pending, &code, Utc::now())?;
//! let secret = PairingSecret::new();
//! let pairing_id = store.record_pairing(fingerprint, &secret)?;
//! // The code is consumed; `secret.secret_hex` goes back to the extension.
//! ```

use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use chrono::{DateTime, Duration, Utc};
use rand::{thread_rng, Rng};

/// TTL of a pairing code shown in the app. After it, the code is refused until
/// the user generates a new one.
pub const CODE_TTL: Duration = Duration::minutes(5);

/// Length of the shared secret in bytes (256 bits, hex-encoded → 64 chars).
pub const SECRET_BYTES: usize = 32;

/// Errors of the pairing layer.
#[derive(Debug, thiserror::Error)]
pub enum PairingError {
    #[error("code invalide ou expiré")]
    InvalidCode,
    #[error("code à 6 chiffres requis")]
    Malformed,
}

/// Pairing code waiting to be used (in-memory, never persisted).
#[derive(Debug, Clone)]
pub struct PendingCode {
    /// The 6 digits shown to the user.
    pub code: String,
    /// Instant after which the code is no longer accepted.
    pub expires_at: DateTime<Utc>,
}

impl PendingCode {
    /// Builds a random 6-digit code with a 5 min TTL.
    #[must_use]
    pub fn new() -> Self {
        Self::with_expiry(Utc::now() + CODE_TTL)
    }

    /// Builds a code with a given expiry (useful for tests).
    #[must_use]
    pub fn with_expiry(expires_at: DateTime<Utc>) -> Self {
        let mut rng = thread_rng();
        // 6 digits as a string, zero-padded if needed (e.g. "042039").
        let n: u32 = rng.gen_range(0..1_000_000);
        Self {
            code: format!("{n:06}"),
            expires_at,
        }
    }

    /// `true` if the code is still valid (not expired).
    #[must_use]
    pub fn is_valid(&self, now: DateTime<Utc>) -> bool {
        now <= self.expires_at
    }
}

impl Default for PendingCode {
    fn default() -> Self {
        Self::new()
    }
}

/// Secret shared by the extension and the desktop app, generated when the
/// code is validated. Stored in clear by the extension (`chrome.storage.local`
/// sandbox), hashed by the app with **Argon2id** (PHC string).
///
/// The PHC string (`$argon2id$v=19$m=...,t=...,p=...$<salt-b64>$<hash-b64>`)
/// already embeds its params and its salt, so `salt_hex` **no longer needs**
/// to be stored separately in the database.
#[derive(Debug, Clone)]
pub struct PairingSecret {
    /// 32-byte secret, hex-encoded (64 ASCII characters).
    pub secret_hex: String,
    /// Argon2id hash of the secret in PHC format, to store on the app side
    /// in `device_pairings.secret_hash`.
    pub secret_hash: String,
}

impl PairingSecret {
    /// Generates a random secret + its Argon2id hash.
    ///
    /// Panics only if the OS RNG behind `SaltString::generate` fails
    /// (which means the OS is in an unrecoverable state).
    #[must_use]
    pub fn new() -> Self {
        let mut rng = thread_rng();
        let mut secret = [0u8; SECRET_BYTES];
        rng.fill(&mut secret);
        let secret_hex = encode_hex(&secret);
        let secret_hash = hash_secret(&secret_hex);
        Self {
            secret_hex,
            secret_hash,
        }
    }

    /// Checks that a clear (hex) secret matches a stored Argon2id PHC hash.
    /// The comparison is constant-time inside `argon2`.
    ///
    /// Returns `false` if the hash is malformed (invalid PHC) or if the
    /// candidate secret does not match.
    #[must_use]
    pub fn verify_against(stored_phc: &str, candidate_hex: &str) -> bool {
        let Ok(parsed) = PasswordHash::new(stored_phc) else {
            return false;
        };
        Argon2::default()
            .verify_password(candidate_hex.as_bytes(), &parsed)
            .is_ok()
    }
}

impl Default for PairingSecret {
    fn default() -> Self {
        Self::new()
    }
}

/// `true` if `stored_hash` looks like an Argon2id PHC hash (prefix
/// `$argon2id$`). The v2 → v3 migration matches the same prefix in SQL
/// (`NOT LIKE '$argon2id$%'`), not with this function.
#[must_use]
pub fn is_argon2id_phc(stored_hash: &str) -> bool {
    stored_hash.starts_with("$argon2id$")
}

/// Checks a submitted code against the expected code, in constant time.
///
/// Returns `Err(Malformed)` if the input is not 6 ASCII digits,
/// `Err(InvalidCode)` if the comparison fails or the code has expired.
pub fn verify_code(
    pending: &PendingCode,
    submitted: &str,
    now: DateTime<Utc>,
) -> Result<(), PairingError> {
    if submitted.len() != 6 || !submitted.chars().all(|c| c.is_ascii_digit()) {
        return Err(PairingError::Malformed);
    }
    if pending.is_valid(now) && constant_time_eq(submitted.as_bytes(), pending.code.as_bytes()) {
        Ok(())
    } else {
        Err(PairingError::InvalidCode)
    }
}

// ─── Private helpers ─────────────────────────────────────────────────────────

/// Argon2id hash (PHC string) of a clear secret. Salt generated by the OS RNG.
///
/// The default params of `argon2 = 0.5` (m=19456, t=2, p=1) match the
/// OWASP recommendation, see
/// <https://cheatsheetseries.owasp.org/cheatsheets/Password_Storage_Cheat_Sheet.html#argon2id>.
fn hash_secret(secret_hex: &str) -> String {
    let salt = SaltString::generate(&mut OsRng);
    Argon2::default()
        .hash_password(secret_hex.as_bytes(), &salt)
        .expect("argon2 hash : OS RNG indisponible (état non récupérable)")
        .to_string()
}

/// Lowercase hex encoding.
fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

/// Constant-time comparison of two byte slices.
///
/// Prevents timing attacks, even locally. In-house implementation:
/// `subtle::ConstantTimeEq` would do the same but adds a dependency.
#[must_use]
fn constant_time_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    let mut acc: u8 = 0;
    for (x, y) in a.iter().zip(b.iter()) {
        acc |= x ^ y;
    }
    acc == 0
}

// ─── Tests ──────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration;

    #[test]
    fn pending_code_is_6_digits() {
        let code = PendingCode::new();
        assert_eq!(code.code.len(), 6);
        assert!(code.code.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn code_is_valid_just_after_creation() {
        let code = PendingCode::new();
        assert!(code.is_valid(Utc::now()));
    }

    #[test]
    fn code_is_invalid_after_ttl() {
        let past = Utc::now() - Duration::seconds(1);
        let code = PendingCode::with_expiry(past);
        assert!(!code.is_valid(Utc::now()));
    }

    #[test]
    fn verify_code_accepts_matching_code() {
        let code = PendingCode::with_expiry(Utc::now() + Duration::minutes(5));
        let res = verify_code(&code, &code.code, Utc::now());
        assert!(res.is_ok());
    }

    #[test]
    fn verify_code_rejects_wrong_code() {
        let code = PendingCode {
            code: "123456".into(),
            expires_at: Utc::now() + Duration::minutes(5),
        };
        let err = verify_code(&code, "654321", Utc::now()).unwrap_err();
        assert!(matches!(err, PairingError::InvalidCode));
    }

    #[test]
    fn verify_code_rejects_expired_code() {
        let code = PendingCode {
            code: "123456".into(),
            expires_at: Utc::now() - Duration::seconds(1),
        };
        let err = verify_code(&code, "123456", Utc::now()).unwrap_err();
        assert!(matches!(err, PairingError::InvalidCode));
    }

    #[test]
    fn verify_code_rejects_malformed() {
        let code = PendingCode {
            code: "123456".into(),
            expires_at: Utc::now() + Duration::minutes(5),
        };
        assert!(matches!(
            verify_code(&code, "abc123", Utc::now()),
            Err(PairingError::Malformed)
        ));
        assert!(matches!(
            verify_code(&code, "12345", Utc::now()),
            Err(PairingError::Malformed)
        ));
        assert!(matches!(
            verify_code(&code, "1234567", Utc::now()),
            Err(PairingError::Malformed)
        ));
    }

    #[test]
    fn pairing_secret_is_64_hex_chars_with_argon2id_phc() {
        let s = PairingSecret::new();
        assert_eq!(s.secret_hex.len(), 64);
        assert!(s.secret_hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert!(
            is_argon2id_phc(&s.secret_hash),
            "secret_hash doit être une PHC Argon2id : {}",
            s.secret_hash
        );
    }

    #[test]
    fn verify_against_accepts_matching_secret() {
        let s = PairingSecret::new();
        assert!(PairingSecret::verify_against(&s.secret_hash, &s.secret_hex));
    }

    #[test]
    fn verify_against_rejects_wrong_secret() {
        let s = PairingSecret::new();
        // Changes 1 character of the secret → the Argon2 check must fail.
        let mut tampered = s.secret_hex.clone();
        tampered.replace_range(0..1, "f");
        if tampered == s.secret_hex {
            tampered.replace_range(0..1, "e");
        }
        assert!(!PairingSecret::verify_against(&s.secret_hash, &tampered));
    }

    #[test]
    fn verify_against_rejects_malformed_phc() {
        // A legacy SHA-256+salt hash is not an Argon2 PHC string → false.
        let s = PairingSecret::new();
        assert!(!PairingSecret::verify_against(
            "not-a-phc-string",
            &s.secret_hex
        ));
        assert!(!PairingSecret::verify_against(
            "0123456789abcdef".repeat(4).as_str(),
            &s.secret_hex
        ));
    }

    #[test]
    fn is_argon2id_phc_detects_legacy_sha256() {
        // v2 → v3 migration: the old hex SHA-256 hashes (64 chars) must
        // never be mistaken for an Argon2id PHC.
        assert!(!is_argon2id_phc(&"a".repeat(64)));
        assert!(is_argon2id_phc(
            "$argon2id$v=19$m=19456,t=2,p=1$c2FsdHNhbHRzYWx0c2FsdA$Zm9v"
        ));
    }

    #[test]
    fn constant_time_eq_basic() {
        assert!(constant_time_eq(b"abc", b"abc"));
        assert!(!constant_time_eq(b"abc", b"abd"));
        assert!(!constant_time_eq(b"abc", b"ab"));
        assert!(constant_time_eq(b"", b""));
    }

    #[test]
    fn two_pending_codes_are_different() {
        // With 10^6 possible values, collisions are rare. Draws 5 and
        // checks that at least 4 are distinct (probabilistic test).
        let codes: Vec<String> = (0..5).map(|_| PendingCode::new().code).collect();
        let unique: std::collections::HashSet<_> = codes.iter().collect();
        assert!(
            unique.len() >= 4,
            "trop de collisions sur 5 tirages : {codes:?}"
        );
    }

    #[test]
    fn two_pairing_secrets_are_different() {
        let s1 = PairingSecret::new();
        let s2 = PairingSecret::new();
        assert_ne!(s1.secret_hex, s2.secret_hex);
        assert_ne!(
            s1.secret_hash, s2.secret_hash,
            "Argon2 doit produire deux PHC strings différentes (sels distincts)"
        );
    }
}
