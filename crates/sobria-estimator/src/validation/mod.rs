//! Cross-validation of the Monte-Carlo engine.
//!
//! See `docs/archive/chantiers/C07-validation-croisee.md` and
//! `docs/methodology/VALIDATION-CROISEE.md`.
//!
//! Two levels:
//! - [`PlausibilityCase`]: order of magnitude (wide range), always on.
//! - [`ReproductionCase`]: reproduction at ±X % of a target computed from the
//!   presets, not a published figure (SOB-209).

use chrono::Utc;
use serde::Serialize;
use sobria_core::{ContextOverhead, EstimationRequest, EstimationResult, Indicator, InputModality};

use crate::{
    distributions::Distribution,
    engine::MonteCarloEngine,
    engine_trait::{find_indicator, EmpreinteMethod},
    engines::ecologits::EcoLogitsEngine,
    error::EstimatorResult,
    params::EstimationParams,
};

pub mod cases;

/// "Order of magnitude" test case (wide range).
#[derive(Debug, Clone, Copy)]
pub struct PlausibilityCase {
    /// Stable case identifier.
    pub id: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Model (preset registry).
    pub model_id: &'static str,
    /// Input tokens.
    pub tokens_in: u32,
    /// Output tokens.
    pub tokens_out: u32,
    /// Electricity emission factor (gCO₂eq/kWh).
    pub if_electrical_g_per_kwh: f64,
    /// Expected (min, max) range for the P50, in gCO₂eq.
    pub expected_range_g_co2eq: (f64, f64),
    /// Documentary reference (paper, EcoLogits doc, etc.).
    pub reference: &'static str,
}

/// Reproduction case: ±X % of a target computed from the presets (SOB-209).
#[derive(Debug, Clone, Copy)]
pub struct ReproductionCase {
    /// Stable identifier.
    pub id: &'static str,
    /// Methodology checked; [`run_reproduction`] builds the matching engine.
    pub method: EmpreinteMethod,
    /// Source the case cites (paper, vendor doc or announcement); the target
    /// itself is computed from the presets (SOB-209).
    pub source_doi_or_url: &'static str,
    /// Target model.
    pub model_id: &'static str,
    /// Input tokens.
    pub tokens_in: u32,
    /// Output tokens.
    pub tokens_out: u32,
    /// Electricity emission factor used in the reference.
    pub if_electrical_g_per_kwh: f64,
    /// PUE used in the reference.
    pub pue: f64,
    /// Target P50 (gCO₂eq).
    pub expected_p50_g_co2eq: f64,
    /// Relative tolerance (e.g. 0.15 for ±15 %).
    pub tolerance: f64,
    /// If `true`, disables the embodied component in our estimator
    /// (Point 0). Useful for usage-only comparisons between methodologies
    /// that allocate embodied differently.
    pub disable_embodied: bool,
    /// Methodology notes.
    pub notes: &'static str,
    /// Input modalities to add to the request (vision, doc,
    /// audio). Empty by default (legacy text cases).
    pub modalities: &'static [InputModality],
    /// System overhead to add to the request. Zeros by
    /// default (legacy text cases).
    pub overhead: ContextOverhead,
}

/// Kind of a validated case.
#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ValidationKind {
    /// Order-of-magnitude test.
    Plausibility,
    /// Strict reproduction test.
    Reproduction,
}

/// Report for a validated case.
#[derive(Debug, Clone, Serialize)]
pub struct ValidationReport {
    /// Case identifier.
    pub case_id: String,
    /// Case kind.
    pub kind: ValidationKind,
    /// `true` if the test passes.
    pub passed: bool,
    /// P50 computed by the engine (gCO₂eq).
    pub computed_p50_g_co2eq: f64,
    /// Expected value (depends on the case kind): a range or a point.
    pub expected: Expectation,
    /// Human-readable message summing up the result.
    pub message: String,
}

/// Expected value.
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum Expectation {
    /// [min, max] range (plausibility).
    Range {
        /// Lower bound.
        min: f64,
        /// Upper bound.
        max: f64,
    },
    /// Point value ± relative tolerance.
    PointWithinTolerance {
        /// Target value.
        target: f64,
        /// Relative tolerance.
        tolerance: f64,
    },
}

/// A text-only request: no datacenter, no modality, no context overhead.
pub(crate) fn request_for(model_id: &str, tokens_in: u32, tokens_out: u32) -> EstimationRequest {
    EstimationRequest {
        model_id: model_id.into(),
        tokens_in,
        tokens_out_estimated: tokens_out,
        datacenter_id: None,
        timestamp: Utc::now(),
        modalities: Vec::new(),
        overhead: sobria_core::ContextOverhead::default(),
    }
}

/// Builds an `EstimationRequest` for a `ReproductionCase`,
/// with its modalities and overhead injected (empty for legacy cases).
fn request_for_reproduction(case: &ReproductionCase) -> EstimationRequest {
    EstimationRequest {
        modalities: case.modalities.to_vec(),
        overhead: case.overhead,
        ..request_for(case.model_id, case.tokens_in, case.tokens_out)
    }
}

fn co2eq_p50(result: &EstimationResult) -> EstimatorResult<f64> {
    find_indicator(result, Indicator::Co2Eq)
        .map(|i| i.interval.p50)
        .ok_or_else(|| crate::error::EstimatorError::Other("CO2eq absent du résultat".into()))
}

/// Runs a plausibility case.
pub fn run_plausibility(case: &PlausibilityCase) -> EstimatorResult<ValidationReport> {
    let params =
        EstimationParams::for_model(case.model_id)?.with_if_electrical(Distribution::Point {
            value: case.if_electrical_g_per_kwh,
        });
    let engine = MonteCarloEngine::new(42).with_n(2_000);
    let result = engine.estimate(
        &request_for(case.model_id, case.tokens_in, case.tokens_out),
        &params,
    )?;
    let p50 = co2eq_p50(&result)?;
    let (min, max) = case.expected_range_g_co2eq;
    let passed = p50 >= min && p50 <= max;
    let message = if passed {
        format!("{} : OK ({p50:.6} ∈ [{min}, {max}] g CO₂eq)", case.id)
    } else {
        format!(
            "{} : KO — P50={p50:.6} hors plage [{min}, {max}] g CO₂eq",
            case.id
        )
    };
    Ok(ValidationReport {
        case_id: case.id.into(),
        kind: ValidationKind::Plausibility,
        passed,
        computed_p50_g_co2eq: p50,
        expected: Expectation::Range { min, max },
        message,
    })
}

/// Runs a strict reproduction case.
///
/// The engine is picked from `case.method`:
/// - `EmpreinteMethod::AfnorSobria` → [`MonteCarloEngine`] (10⁴ draws)
/// - `EmpreinteMethod::EcoLogits`   → [`EcoLogitsEngine`] (deterministic)
pub fn run_reproduction(case: &ReproductionCase) -> EstimatorResult<ValidationReport> {
    let mut params = EstimationParams::for_model(case.model_id)?
        .with_if_electrical(Distribution::Point {
            value: case.if_electrical_g_per_kwh,
        })
        .with_pue(Distribution::Point { value: case.pue });
    if case.disable_embodied {
        // Usage-only comparison: we zero out our embodied amortisation
        // to stay consistent with the target value (which leaves embodied
        // out or includes it through a different formula).
        params = params.with_embodied(Distribution::Point { value: 0.0 });
    }
    let request = request_for_reproduction(case);
    let result = match case.method {
        EmpreinteMethod::AfnorSobria => {
            let engine = MonteCarloEngine::new(42).with_n(10_000);
            engine.estimate(&request, &params)?
        },
        EmpreinteMethod::EcoLogits => {
            let engine = EcoLogitsEngine::new(42);
            engine.estimate(&request, &params)?
        },
    };
    let p50 = co2eq_p50(&result)?;
    let relative_error = (p50 - case.expected_p50_g_co2eq).abs() / case.expected_p50_g_co2eq;
    let passed = relative_error <= case.tolerance;
    let message = if passed {
        format!(
            "{} : OK (P50={p50:.4}, cible={:.4}, err={:.1}%, tol={:.1}%)",
            case.id,
            case.expected_p50_g_co2eq,
            relative_error * 100.0,
            case.tolerance * 100.0
        )
    } else {
        format!(
            "{} : KO — P50={p50:.4}, cible={:.4}, err={:.1}% > tol={:.1}%",
            case.id,
            case.expected_p50_g_co2eq,
            relative_error * 100.0,
            case.tolerance * 100.0
        )
    };
    Ok(ValidationReport {
        case_id: case.id.into(),
        kind: ValidationKind::Reproduction,
        passed,
        computed_p50_g_co2eq: p50,
        expected: Expectation::PointWithinTolerance {
            target: case.expected_p50_g_co2eq,
            tolerance: case.tolerance,
        },
        message,
    })
}

/// Runs all known plausibility cases.
#[must_use]
pub fn run_all_plausibility() -> Vec<ValidationReport> {
    cases::PLAUSIBILITY_CASES
        .iter()
        .filter_map(|c| run_plausibility(c).ok())
        .collect()
}

/// Runs all known reproduction cases.
#[must_use]
pub fn run_all_reproduction() -> Vec<ValidationReport> {
    cases::REPRODUCTION_CASES
        .iter()
        .filter_map(|c| run_reproduction(c).ok())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_plausibility_cases_pass() {
        for case in cases::PLAUSIBILITY_CASES {
            let report = run_plausibility(case)
                .unwrap_or_else(|e| panic!("erreur de validation pour {} : {e}", case.id));
            assert!(report.passed, "{}", report.message);
        }
    }

    #[test]
    fn run_all_plausibility_returns_at_least_five() {
        let reports = run_all_plausibility();
        assert!(
            reports.len() >= 5,
            "attendu ≥ 5 rapports, reçu {}",
            reports.len()
        );
        for r in &reports {
            assert!(r.passed, "{}", r.message);
        }
    }

    #[test]
    fn at_least_three_reproduction_cases_registered() {
        assert!(
            cases::REPRODUCTION_CASES.len() >= 3,
            "attendu ≥ 3 cas de reproduction, reçu {}",
            cases::REPRODUCTION_CASES.len()
        );
    }

    /// Checks that each [`ReproductionCase`] passes its tolerance against
    /// the engine selected by `case.method`.
    ///
    /// For `EcoLogits` cases: the Rust port must reproduce the official
    /// formulas within <1 % (direct port → the expected gap is purely
    /// float64 numerical).
    #[test]
    fn all_reproduction_cases_pass() {
        for case in cases::REPRODUCTION_CASES {
            let report = run_reproduction(case)
                .unwrap_or_else(|e| panic!("erreur de validation pour {} : {e}", case.id));
            assert!(report.passed, "{}", report.message);
        }
    }

    #[test]
    fn all_reproduction_cases_cite_a_doi_or_url() {
        for case in cases::REPRODUCTION_CASES {
            let s = case.source_doi_or_url;
            assert!(
                s.starts_with("https://") || s.starts_with("doi:") || s.starts_with("10."),
                "cas {} : source non citée correctement ({s:?})",
                case.id
            );
        }
    }
}
