//! Typed errors of the `sobria-team-aggregator` library.
//!
//! CLAUDE.md §4.1: `thiserror` for library errors, `anyhow` for the CLI paths.
//! HTTP errors are [`crate::server::error::ApiError`].

use thiserror::Error;

/// Error of the `sobria-team-aggregator` daemon.
#[derive(Debug, Error)]
pub enum AggregatorError {
    /// I/O failure (reading the cert, writing the data dir, etc.).
    #[error("erreur I/O : {0}")]
    Io(#[from] std::io::Error),

    /// SQLite migration or query failure.
    #[error("erreur SQLite : {0}")]
    Sqlite(#[from] rusqlite::Error),

    /// Failure generating / parsing the self-signed TLS certificate.
    #[error("erreur TLS : {0}")]
    Tls(String),

    /// Argon2id hash / verification failure.
    #[error("erreur cryptographique : {0}")]
    Crypto(String),

    /// Configuration error (invalid data dir, missing cert, etc.).
    #[error("configuration invalide : {0}")]
    Config(String),

    /// The data dir already exists and `init` was called without `--force`.
    #[error("data dir déjà initialisé : {0} (utiliser `--force` pour réinitialiser)")]
    AlreadyInitialized(String),

    /// Internal error (business validation, unexpected format, etc.).
    #[error("erreur interne : {0}")]
    Internal(String),
}

/// Convenience alias for internal fallible paths.
pub type AggregatorResult<T> = Result<T, AggregatorError>;
