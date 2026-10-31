//! Génération de secrets cryptographiques (JWT signing key, etc.).
//!
//! On utilise l'OS RNG via `rand::rngs::OsRng` (même approche que C27 pour
//! le secret de pairing). Les octets sont encodés en hex pour stockage texte
//! dans la table `config` SQLite.

use rand::RngCore;

/// Génère N octets aléatoires (OS RNG) encodés en hex lowercase.
#[must_use]
pub fn random_hex(n_bytes: usize) -> String {
    let mut buf = vec![0u8; n_bytes];
    rand::rngs::OsRng.fill_bytes(&mut buf);
    encode_hex(&buf)
}

/// Génère un secret JWT 32 octets (256 bits) en hex (64 chars).
#[must_use]
pub fn jwt_signing_key() -> String {
    random_hex(32)
}

/// Code aléatoire 12 chiffres ASCII via OS RNG.
///
/// Entropie ≈ 40 bits (10^12 = 2^39.86) — suffisant car les codes sont
/// révoqués après usage et Argon2id rend tout brute force serveur lent.
#[must_use]
pub fn enrollment_code() -> String {
    let mut bytes = [0u8; 16];
    rand::rngs::OsRng.fill_bytes(&mut bytes);
    // 16 octets → u128 ; on prend modulo 10^12 pour un nombre 12 chiffres.
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
