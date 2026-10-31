//! Implementations of [`crate::engine_trait::EmpreinteEngine`].
//!
//! Each submodule is one embedded methodology:
//! - [`ecologits`]: direct port of EcoLogits 2026-01 (CC BY-SA 4.0).
//!
//! The AFNOR SPEC 2314 methodology (Sobr.ia) is implemented in
//! [`crate::engine::MonteCarloEngine`] (at the crate root, for historical
//! reasons).

pub mod ecologits;
pub mod factory;
