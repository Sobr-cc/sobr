//! Cryptographic secret generation (JWT signing key, etc.).
//!
//! Uses the OS RNG through `rand::rngs::OsRng` (same approach as C27 for
//! the pairing secret). Bytes are hex-encoded for text storage in the
//! SQLite `config` table.

use rand::RngCore;

/// Generates N random bytes (OS RNG) encoded as lowercase hex.
#[must_use]
pub fn random_hex(n_bytes: usize) -> String {
    let mut buf = vec![0u8; n_bytes];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    encode_hex(&buf)
}

/// Generates a 32-byte (256-bit) JWT secret as hex (64 chars).
#[must_use]
pub fn jwt_signing_key() -> String {
    random_hex(32)
}

/// Random 12-digit ASCII code from the OS RNG.
///
/// Entropy ≈ 40 bits (10^12 = 2^39.86) — enough because codes are revoked
/// after use and Argon2id makes any server-side brute force slow.
#[must_use]
pub fn enrollment_code() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    // 16 bytes → u128; take modulo 10^12 for a 12-digit number.
    let mut acc: u128 = 0;
    for b in &bytes {
        acc = acc.wrapping_mul(31).wrapping_add(u128::from(*b));
    }
    let n: u64 = (acc % 1_000_000_000_000u128) as u64;
    format!("{n:012}")
}

fn encode_hex(bytes: &[u8]) -> String {
    use std::fmt::Write;
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn jwt_signing_key_is_64_hex_chars() {
        let k = jwt_signing_key();
        assert_eq!(k.len(), 64);
        assert!(k.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn two_jwt_keys_are_different() {
        let a = jwt_signing_key();
        let b = jwt_signing_key();
        assert_ne!(a, b);
    }

    #[test]
    fn random_hex_respects_length() {
        assert_eq!(random_hex(16).len(), 32);
        assert_eq!(random_hex(0).len(), 0);
    }
}
