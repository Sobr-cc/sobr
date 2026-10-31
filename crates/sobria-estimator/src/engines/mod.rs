//! Implémentations de [`crate::engine_trait::EmpreinteEngine`].
//!
//! Chaque sous-module correspond à une méthodologie embarquée :
//! - [`ecologits`] : port direct EcoLogits 2026-01 (CC BY-SA 4.0).
//!
//! La méthodologie AFNOR SPEC 2314 (Sobr.ia) est implémentée dans
//! [`crate::engine::MonteCarloEngine`] (at the crate root, for historical
//! reasons).

pub mod ecologits;
pub mod factory;
