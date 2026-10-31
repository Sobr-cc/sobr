//! Public errors of `sobria-estimator`.

use thiserror::Error;

/// `Result` alias for the crate.
pub type EstimatorResult<T> = Result<T, EstimatorError>;

/// Public error categories.
#[derive(Debug, Error)]
pub enum EstimatorError {
    /// Non-conforming input data.
    #[error("schéma : {0}")]
    Schema(String),

    /// Internal validation failure.
    #[error("validation : {0}")]
    Validation(String),

    /// Arbitrary error.
    #[error("erreur : {0}")]
    Other(String),
}
