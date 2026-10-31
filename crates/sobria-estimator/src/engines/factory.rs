//! Factory that builds the right engine for an [`EmpreinteMethod`].

use sobria_core::DEFAULT_SEED;

use crate::{
    engine::MonteCarloEngine,
    engine_trait::{EmpreinteEngine, EmpreinteMethod},
    engines::ecologits::EcoLogitsEngine,
};

/// Builds the engine matching a methodology.
///
/// Every engine uses the fixed seed `DEFAULT_SEED` (42) for reproducibility;
/// nothing reads `SOBRIA_SEED` yet (SOB-84).
#[must_use]
pub fn engine_for(method: EmpreinteMethod) -> Box<dyn EmpreinteEngine> {
    match method {
        EmpreinteMethod::AfnorSobria => Box::new(MonteCarloEngine::new(DEFAULT_SEED)),
        EmpreinteMethod::EcoLogits => Box::new(EcoLogitsEngine::new(DEFAULT_SEED)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn factory_returns_correct_method() {
        for &method in &[EmpreinteMethod::AfnorSobria, EmpreinteMethod::EcoLogits] {
            let eng = engine_for(method);
            assert_eq!(eng.method(), method);
        }
    }

    #[test]
    fn factory_engines_expose_methodology_info() {
        for &method in &[EmpreinteMethod::AfnorSobria, EmpreinteMethod::EcoLogits] {
            let eng = engine_for(method);
            let info = eng.methodology_info();
            assert_eq!(info.method, method);
            assert_ne!(info.display_name, "");
        }
    }
}
