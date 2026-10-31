//! Common trait for every LLM footprint methodology built into
//! Sobr.ia (ADR-0012).
//!
//! Sobr.ia v1.0+ offers a **catalogue of scientific methodologies**
//! the user can choose from:
//!
//! - **AFNOR SPEC 2314 (Sobr.ia)**: linear-per-token formula + Monte-Carlo,
//!   Sobr.ia's own method, aligned on the French AFNOR SPEC 2314 framework.
//! - **EcoLogits 2026-01**: direct port of the EcoLogits formulas (Rincé &
//!   Banse 2025, [DOI:10.21105/joss.07471](https://doi.org/10.21105/joss.07471),
//!   licence CC BY-SA 4.0).
//! - *(planned for v1.1+)* BoaVizta, HF AI Energy Score, Custom user engine.
//!
//! ## UX
//!
//! The user picks **one** default methodology in
//! `Settings → Méthodologies`. They can also turn on other
//! methodologies as references: their results appear
//! in a "Voir aussi" panel next to the main result.
//!
//! The audit ledger always records the methodology used for
//! each estimate (`method` field in the `audit_entries` table).

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sobria_core::{
    DistributionBins, EstimationRequest, EstimationResult, Hypothesis, Indicator, IndicatorValue,
    UncertaintyInterval,
};

use crate::{distributions::Distribution, error::EstimatorResult, params::EstimationParams};

/// Re-export of the enum type defined in `sobria-core::methodology`.
///
/// The central type lives in `sobria-core` so it can be used
/// in `EstimationResult` (serialised in the audit ledger) and in the
/// user preferences **without** creating a circular dependency on
/// `sobria-estimator`.
pub use sobria_core::EmpreinteMethod;

/// Calibration status of a methodology within Sobr.ia.
///
/// Distinct from [`crate::model_presets::CalibrationStatus`] (which applies
/// to a *model* in a *preset*): here it is the status of the
/// *methodology* itself.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MethodologyCalibration {
    /// Peer-reviewed methodology, reproduced within <1 % of the published
    /// reference implementation (automatic cross-check test in CI).
    PeerReviewedReproduced,
    /// Public reference methodology, implemented by Sobr.ia and checked
    /// for plausibility (orders of magnitude); the numeric calibration
    /// is in progress.
    PublicMethodCalibrationPending,
    /// Methodology built in for exploration / teaching; treat its values
    /// as indicative only.
    Indicative,
}

/// Static description of a built-in methodology.
///
/// Returned by [`AVAILABLE_METHODS`] at runtime to show the
/// catalogue in `Settings → Méthodologies`.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub struct MethodologyInfo {
    /// Stable identifier.
    pub method: EmpreinteMethod,
    /// Name shown in the UI.
    pub display_name: &'static str,
    /// Short description (1-2 sentences) for a non-technical audience.
    pub short_description: &'static str,
    /// DOI or URL of the academic / open-source reference.
    pub reference_url: &'static str,
    /// Normalised DOI identifier if available (otherwise `None`).
    pub doi: Option<&'static str>,
    /// License of the published methodology.
    pub license: &'static str,
    /// Calibration status in Sobr.ia.
    pub calibration: MethodologyCalibration,
    /// Publication year of the reference methodology.
    pub year_published: u16,
    /// Organisation that maintains the reference methodology.
    pub maintained_by: &'static str,
}

/// Every methodology Sobr.ia can run.
///
/// To add one:
/// 1. Extend the [`EmpreinteMethod`] enum.
/// 2. Add the entry here.
/// 3. Implement [`EmpreinteEngine`] in `crates/sobria-estimator/src/engines/`.
/// 4. Wire it into the [`crate::engine_for`] factory.
pub static AVAILABLE_METHODS: &[MethodologyInfo] = &[
    MethodologyInfo {
        method: EmpreinteMethod::AfnorSobria,
        display_name: "AFNOR SPEC 2314 — Sobr.ia",
        short_description: "Référentiel français de mesure de l'empreinte LLM. \
                            Formule linéaire-par-token + Monte-Carlo N=10⁴.",
        reference_url: "https://norminfo.afnor.org/norme/AFNOR%20SPEC%202314/",
        doi: None,
        license: "AFNOR SPEC publique ; code Sobr.ia sous licence MIT",
        calibration: MethodologyCalibration::PublicMethodCalibrationPending,
        year_published: 2024,
        maintained_by: "Sobr.ia (impl.) / AFNOR (spec)",
    },
    MethodologyInfo {
        method: EmpreinteMethod::EcoLogits,
        display_name: "EcoLogits 2026-01",
        short_description: "Méthodologie open peer-reviewed, port direct des \
                            formules Rincé & Banse (JOSS 2025). Référence \
                            internationale de l'estimation LLM.",
        reference_url: "https://ecologits.ai/latest/methodology/llm_inference/",
        doi: Some("10.21105/joss.07471"),
        license: "CC BY-SA 4.0 (méthodologie) ; code Sobr.ia sous licence MIT",
        calibration: MethodologyCalibration::PeerReviewedReproduced,
        year_published: 2025,
        maintained_by: "GenAI Impact (Rincé & Banse)",
    },
];

/// Common trait of the footprint estimation engines.
///
/// Every methodology built into Sobr.ia implements this trait, so the
/// `sobria-app` layer can call the AFNOR or EcoLogits engine
/// interchangeably, depending on the user preference.
pub trait EmpreinteEngine: Send + Sync {
    /// Stable identifier of the methodology.
    fn method(&self) -> EmpreinteMethod;

    /// Metadata for UI display / the challenge application file.
    ///
    /// # Panics
    ///
    /// Never: [`AVAILABLE_METHODS`] lists every [`EmpreinteMethod`].
    fn methodology_info(&self) -> &'static MethodologyInfo {
        info_for(self.method())
    }

    /// Runs the estimation for a request + a parameter set.
    ///
    /// Not every methodology honours every `EstimationParams` field
    /// (EcoLogits has its own internal coefficients); see the
    /// documentation of each implementation.
    fn estimate(
        &self,
        request: &EstimationRequest,
        params: &EstimationParams,
    ) -> EstimatorResult<EstimationResult>;
}

/// Returns the info of a methodology by identifier.
///
/// # Panics
///
/// Never: [`AVAILABLE_METHODS`] lists every [`EmpreinteMethod`].
#[must_use]
pub fn info_for(method: EmpreinteMethod) -> &'static MethodologyInfo {
    AVAILABLE_METHODS
        .iter()
        .find(|m| m.method == method)
        .expect("AVAILABLE_METHODS doit couvrir tous les EmpreinteMethod variants")
}

/// The CO₂eq, energy and water indicators, in the order every engine returns them.
pub(crate) fn indicator_values(
    co2eq: (UncertaintyInterval, Option<DistributionBins>),
    energy: (UncertaintyInterval, Option<DistributionBins>),
    water: (UncertaintyInterval, Option<DistributionBins>),
) -> Vec<IndicatorValue> {
    [
        (Indicator::Co2Eq, co2eq),
        (Indicator::Energy, energy),
        (Indicator::Water, water),
    ]
    .into_iter()
    .map(|(indicator, (interval, bins))| IndicatorValue {
        indicator,
        interval,
        unit: indicator.default_unit().into(),
        bins,
    })
    .collect()
}

pub(crate) fn distribution_hypothesis(key: &str, dist: &Distribution, source: &str) -> Hypothesis {
    Hypothesis {
        key: key.into(),
        value: serde_json::to_value(dist).unwrap_or(serde_json::Value::Null),
        source: source.into(),
    }
}

pub(crate) fn find_indicator(
    result: &EstimationResult,
    indicator: Indicator,
) -> Option<&IndicatorValue> {
    result.indicators.iter().find(|i| i.indicator == indicator)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn available_methods_covers_all_variants() {
        // Every enum variant must have an entry in AVAILABLE_METHODS.
        // If a variant is added to the enum but the registry is forgotten,
        // this test fails.
        for &method in EmpreinteMethod::all() {
            assert!(
                AVAILABLE_METHODS.iter().any(|m| m.method == method),
                "AVAILABLE_METHODS ne couvre pas {method:?}"
            );
        }
    }

    #[test]
    fn info_for_returns_matching_entry() {
        for &method in EmpreinteMethod::all() {
            let info = info_for(method);
            assert_eq!(info.method, method);
        }
    }

    #[test]
    fn ecologits_entry_has_doi() {
        let info = info_for(EmpreinteMethod::EcoLogits);
        assert_eq!(info.doi, Some("10.21105/joss.07471"));
        assert!(info.reference_url.starts_with("https://"));
    }
}
