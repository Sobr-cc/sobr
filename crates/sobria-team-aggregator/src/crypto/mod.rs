//! Cryptographic primitives of the `sobria-team-aggregator`.
//!
//! - [`password`]: Argon2id PHC hashes (admin passwords, enrollment codes, refresh tokens).
//! - [`secret`]: OS RNG secrets (JWT signing key, enrollment codes).
//! - [`tls`]: self-signed certificate through `rcgen` (ring backend).

pub mod password;
pub mod secret;
pub mod tls;
