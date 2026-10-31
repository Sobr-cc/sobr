//! « Et si...? » ("What if...?") simulator, chantier C11, module M13.
//!
//! Compares a baseline with N scenarios that pin some parameters to
//! fixed values (model, PUE, electricity mix, etc.), and projects
//! these changes over 12 months with a geometric adoption-growth
//! assumption.
//!
//! See `docs/archive/chantiers/C11-simulateur-et-si.md` for the spec.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sobria_core::{EstimationRequest, EstimationResult, Indicator};

use crate::{
    distributions::Distribution,
    engine_trait::find_indicator,
    error::{EstimatorError, EstimatorResult},
    forecast,
    params::EstimationParams,
};

/// Maximum number of scenarios allowed per request (anti-abuse).
pub const MAX_SCENARIOS: usize = 20;

/// Maximum projection horizon (months).
pub const MAX_FORECAST_MONTHS: u32 = 60;

/// Partial overrides applied to the baseline request and parameters.
///
/// Every field is an `Option`: `None` keeps the baseline value.
/// When a field is `Some`, it becomes a `Distribution::Point` (deterministic
/// value), to explore "what if PUE were exactly 1.1?".
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ParamOverrides {
    /// Alternative model (then uses `EstimationParams::for_model`).
    pub model_id: Option<String>,
    /// Alternative output tokens (affects the E_compute calculation).
    pub tokens_out: Option<u32>,
    /// PUE forced to this value (deterministic).
    pub pue: Option<f64>,
    /// Forced electricity emission factor (gCO₂eq/kWh).
    pub if_electrical_g_per_kwh: Option<f64>,
    /// Forced embodied emissions per request (gCO₂eq).
    pub embodied_g_per_request: Option<f64>,
    /// Forced WUE (L/kWh).
    pub wue_l_per_kwh: Option<f64>,
}

impl ParamOverrides {
    /// `true` if at least one field is set.
    #[must_use]
    pub fn any(&self) -> bool {
        self.model_id.is_some()
            || self.tokens_out.is_some()
            || self.pue.is_some()
            || self.if_electrical_g_per_kwh.is_some()
            || self.embodied_g_per_request.is_some()
            || self.wue_l_per_kwh.is_some()
    }
}

/// A named scenario with its overrides.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct Scenario {
    /// Label shown in the UI (e.g. « France 100% renouvelable »).
    pub label: String,
    /// Applied overrides. If empty, the scenario matches the baseline.
    #[serde(default)]
    pub overrides: ParamOverrides,
}

/// Configuration of a 12-month projection (or another horizon).
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct ForecastConfig {
    /// Horizon in months. Capped at `MAX_FORECAST_MONTHS`.
    pub months: u32,
    /// Monthly growth as a percentage (e.g. `5.0` = +5%/month).
    /// Negative values allowed (declining usage).
    pub monthly_growth_pct: f64,
    /// Baseline volume: number of prompts per day in month 0.
    pub base_volume_per_day: f64,
}

/// Simulation request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct SimulationRequest {
    /// Reference estimate, used as the point of comparison.
    pub baseline: EstimationRequest,
    /// Scenarios to evaluate (1..= `MAX_SCENARIOS`).
    pub scenarios: Vec<Scenario>,
    /// Optional projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forecast: Option<ForecastConfig>,
}

/// Result of one scenario.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ScenarioOutcome {
    /// Label taken from the input scenario.
    pub label: String,
    /// Full estimation result (with bins).
    pub result: EstimationResult,
    /// Δ from the baseline P50, in gCO₂eq (can be negative).
    pub delta_co2eq_g: f64,
    /// Relative Δ from the baseline P50, as a percentage.
    pub delta_pct: f64,
}

/// Result of a 12-month projection.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ForecastResult {
    /// Effective horizon (can be ≤ the requested months when capped).
    pub months: u32,
    /// Baseline volume per day used to compute the masses.
    pub base_volume_per_day: f64,
    /// Monthly growth used (percentage).
    pub monthly_growth_pct: f64,
    /// Baseline monthly series in gCO₂eq (length = months).
    pub baseline_monthly_co2eq_g: Vec<f64>,
    /// Baseline total over `months` (not a calendar year despite the name,
    /// SOB-210), in gCO₂eq.
    pub baseline_annual_co2eq_g: f64,
    /// Total over `months` per scenario, in the order of
    /// `SimulationRequest.scenarios`.
    pub scenarios_annual_co2eq_g: Vec<f64>,
}

/// Overall result of a simulation.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SimulationResult {
    /// Baseline estimate (incl. distribution bins).
    pub baseline: EstimationResult,
    /// One outcome per scenario, in request order.
    pub scenarios: Vec<ScenarioOutcome>,
    /// Optional projection.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forecast: Option<ForecastResult>,
}

// ─────────────────────────────────────────────────────────────────────────────
// simulate()
// ─────────────────────────────────────────────────────────────────────────────

/// Runs the baseline + every scenario + the projection.
///
/// `engine` runs the methodology the user chose. `baseline_params` comes from
/// the caller, which can apply a datacenter override (PUE, IF, WUE); the
/// scenarios start from it, except a `model_id` override, which rebuilds the
/// params from that model's preset.
///
/// **Errors**:
/// - `Schema` if there are more than `MAX_SCENARIOS` scenarios,
/// - `Schema` if the forecast is out of bounds,
/// - `Schema` if two scenarios share the same `label` (avoids UI confusion),
/// - propagates the errors of `engine.estimate` (unknown model, etc.).
pub fn simulate(
    engine: &dyn crate::engine_trait::EmpreinteEngine,
    request: &SimulationRequest,
    baseline_params: &EstimationParams,
) -> EstimatorResult<SimulationResult> {
    forecast::check_scenario_count(request.scenarios.len(), MAX_SCENARIOS)?;
    let mut seen_labels = std::collections::HashSet::new();
    for s in &request.scenarios {
        if !seen_labels.insert(s.label.as_str()) {
            return Err(forecast::duplicate_label(&s.label));
        }
    }
    if let Some(f) = &request.forecast {
        validate_forecast(f)?;
    }

    let baseline = engine.estimate(&request.baseline, baseline_params)?;
    let baseline_co2_p50 = co2_p50(&baseline)?;

    let mut outcomes: Vec<ScenarioOutcome> = Vec::with_capacity(request.scenarios.len());
    let mut scenario_p50s = Vec::with_capacity(request.scenarios.len());
    for scenario in &request.scenarios {
        let (req_derived, params_derived) =
            apply_overrides(&request.baseline, baseline_params, &scenario.overrides)?;
        let result = engine.estimate(&req_derived, &params_derived)?;
        let p50 = co2_p50(&result)?;
        scenario_p50s.push(p50);
        let delta = p50 - baseline_co2_p50;
        let delta_pct = if baseline_co2_p50.abs() > f64::EPSILON {
            (delta / baseline_co2_p50) * 100.0
        } else {
            0.0
        };
        outcomes.push(ScenarioOutcome {
            label: scenario.label.clone(),
            result,
            delta_co2eq_g: delta,
            delta_pct,
        });
    }

    let forecast = request.forecast.as_ref().map(|f| {
        let baseline_monthly = monthly_series(baseline_co2_p50, f);
        let baseline_annual: f64 = baseline_monthly.iter().sum();
        let scenarios_annual: Vec<f64> = scenario_p50s
            .iter()
            .map(|&p50| monthly_series(p50, f).iter().sum())
            .collect();
        ForecastResult {
            months: f.months,
            base_volume_per_day: f.base_volume_per_day,
            monthly_growth_pct: f.monthly_growth_pct,
            baseline_monthly_co2eq_g: baseline_monthly,
            baseline_annual_co2eq_g: baseline_annual,
            scenarios_annual_co2eq_g: scenarios_annual,
        }
    });

    Ok(SimulationResult {
        baseline,
        scenarios: outcomes,
        forecast,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────────────────────

fn validate_forecast(f: &ForecastConfig) -> EstimatorResult<()> {
    forecast::check_months(f.months, MAX_FORECAST_MONTHS)?;
    if !forecast::growth_in_bounds(f.monthly_growth_pct) {
        return Err(EstimatorError::Schema(format!(
            "monthly_growth_pct {} hors bornes [-50, 50]",
            f.monthly_growth_pct
        )));
    }
    forecast::check_volume_per_day(f.base_volume_per_day)
}

fn co2_p50(result: &EstimationResult) -> EstimatorResult<f64> {
    find_indicator(result, Indicator::Co2Eq)
        .map(|i| i.interval.p50)
        .ok_or_else(|| EstimatorError::Schema("CO2eq indicator missing".into()))
}

/// Builds a derived request and parameters by applying the overrides.
fn apply_overrides(
    baseline_req: &EstimationRequest,
    baseline_params: &EstimationParams,
    overrides: &ParamOverrides,
) -> EstimatorResult<(EstimationRequest, EstimationParams)> {
    // Request: replace tokens_out and/or model_id if asked.
    let mut req = baseline_req.clone();
    if let Some(t) = overrides.tokens_out {
        req.tokens_out_estimated = t;
    }
    if let Some(model) = &overrides.model_id {
        req.model_id.clone_from(model);
    }

    // Params: start from those of the effective model_id (possibly changed),
    // then apply the scalar overrides as Point distributions.
    let mut params = if let Some(model) = &overrides.model_id {
        EstimationParams::for_model(model)?
    } else {
        *baseline_params
    };
    if let Some(v) = overrides.pue {
        params.pue = Distribution::Point { value: v };
    }
    if let Some(v) = overrides.if_electrical_g_per_kwh {
        params.if_electrical_g_per_kwh = Distribution::Point { value: v };
    }
    if let Some(v) = overrides.embodied_g_per_request {
        params.embodied_g_per_request = Distribution::Point { value: v };
    }
    if let Some(v) = overrides.wue_l_per_kwh {
        params.wue_l_per_kwh = Distribution::Point { value: v };
    }
    params.validate()?;
    Ok((req, params))
}

/// Computes the aggregated monthly gCO₂eq series from a per-request P50.
///
/// `mois_n = (P50 × volume_jour × 30) × (1 + growth)^n`.
fn monthly_series(p50_per_req_g: f64, cfg: &ForecastConfig) -> Vec<f64> {
    let base_monthly = p50_per_req_g * cfg.base_volume_per_day * forecast::DAYS_PER_MONTH;
    let growth = forecast::growth_factor(cfg.monthly_growth_pct);
    (0..cfg.months)
        .map(|n| forecast::grown(base_monthly, growth, n))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MonteCarloEngine;

    fn baseline_request() -> EstimationRequest {
        crate::validation::request_for("gpt-4o-mini", 100, 500)
    }

    fn sim_request_with(scenarios: Vec<Scenario>) -> SimulationRequest {
        SimulationRequest {
            baseline: baseline_request(),
            scenarios,
            forecast: None,
        }
    }

    /// Test helper: builds the params of the request's baseline and
    /// delegates to `simulate`. Keeps the internal tests short after
    /// the signature change to `simulate(engine, req,
    /// baseline_params)`.
    fn sim(
        engine: &dyn crate::engine_trait::EmpreinteEngine,
        req: &SimulationRequest,
    ) -> EstimatorResult<SimulationResult> {
        let params = EstimationParams::for_model(&req.baseline.model_id)?;
        simulate(engine, req, &params)
    }

    #[test]
    fn baseline_only_runs_without_scenarios() {
        let engine = MonteCarloEngine::default();
        let res = sim(&engine, &sim_request_with(vec![])).unwrap();
        assert_eq!(res.scenarios.len(), 0);
        assert_eq!(res.baseline.indicators.len(), 3);
    }

    #[test]
    fn scenario_with_pue_override_changes_p50() {
        let engine = MonteCarloEngine::new(42);
        let scenarios = vec![
            Scenario {
                label: "PUE bas (1.05)".into(),
                overrides: ParamOverrides {
                    pue: Some(1.05),
                    ..Default::default()
                },
            },
            Scenario {
                label: "PUE haut (1.6)".into(),
                overrides: ParamOverrides {
                    pue: Some(1.6),
                    ..Default::default()
                },
            },
        ];
        let res = sim(&engine, &sim_request_with(scenarios)).unwrap();
        assert_eq!(res.scenarios.len(), 2);
        let pue_low = res.scenarios[0].result.indicators[0].interval.p50;
        let pue_high = res.scenarios[1].result.indicators[0].interval.p50;
        assert!(
            pue_low < pue_high,
            "PUE 1.05 ({pue_low}) doit être < PUE 1.6 ({pue_high})"
        );
        // Delta computed correctly.
        let baseline_p50 = res.baseline.indicators[0].interval.p50;
        assert!((res.scenarios[0].delta_co2eq_g - (pue_low - baseline_p50)).abs() < 1e-9);
    }

    #[test]
    fn scenario_with_if_override_drastically_changes_p50() {
        // France (56 g/kWh) vs coal-heavy USA (~600 g/kWh).
        //
        // We force `embodied = 0` in both scenarios to isolate the
        // electricity-mix contribution. Without it, for a small model
        // like gpt-4o-mini on 100+500 tokens, embodied (~0.002 g)
        // would dominate the electrical part (~0.00001 g) and hide the
        // coal/FR IF ratio. This finding is a methodology lesson
        // surfaced in the M13 UI.
        let engine = MonteCarloEngine::new(42);
        let scenarios = vec![
            Scenario {
                label: "Mix France".into(),
                overrides: ParamOverrides {
                    if_electrical_g_per_kwh: Some(56.0),
                    embodied_g_per_request: Some(0.0),
                    ..Default::default()
                },
            },
            Scenario {
                label: "Mix charbon".into(),
                overrides: ParamOverrides {
                    if_electrical_g_per_kwh: Some(600.0),
                    embodied_g_per_request: Some(0.0),
                    ..Default::default()
                },
            },
        ];
        let res = sim(&engine, &sim_request_with(scenarios)).unwrap();
        let fr = res.scenarios[0].result.indicators[0].interval.p50;
        let coal = res.scenarios[1].result.indicators[0].interval.p50;
        assert!(
            coal > fr * 2.0,
            "charbon ({coal}) doit être >2× France ({fr})"
        );
    }

    #[test]
    fn scenario_with_model_override_uses_new_params() {
        let engine = MonteCarloEngine::new(42);
        let scenarios = vec![Scenario {
            label: "Sonnet à la place de mini".into(),
            overrides: ParamOverrides {
                model_id: Some("claude-3-5-sonnet".into()),
                ..Default::default()
            },
        }];
        let res = sim(&engine, &sim_request_with(scenarios)).unwrap();
        // model_id replaced → expected impact on P50 (likely, but not tested as
        // an absolute value: we only check that it runs and that the delta is
        // computed).
        let outcome = &res.scenarios[0];
        assert!(outcome.delta_co2eq_g.is_finite());
        assert!(outcome.delta_pct.is_finite());
    }

    #[test]
    fn unknown_model_in_scenario_returns_error() {
        let engine = MonteCarloEngine::new(42);
        let scenarios = vec![Scenario {
            label: "Modèle qui n'existe pas".into(),
            overrides: ParamOverrides {
                model_id: Some("does-not-exist".into()),
                ..Default::default()
            },
        }];
        let err = sim(&engine, &sim_request_with(scenarios)).unwrap_err();
        assert!(
            format!("{err:?}").to_lowercase().contains("inconnu")
                || format!("{err:?}").to_lowercase().contains("unknown")
        );
    }

    #[test]
    fn duplicate_scenario_labels_rejected() {
        let engine = MonteCarloEngine::new(42);
        let scenarios = vec![
            Scenario {
                label: "même".into(),
                overrides: ParamOverrides::default(),
            },
            Scenario {
                label: "même".into(),
                overrides: ParamOverrides::default(),
            },
        ];
        let err = sim(&engine, &sim_request_with(scenarios)).unwrap_err();
        assert!(format!("{err}").contains("doublon"));
    }

    #[test]
    fn too_many_scenarios_rejected() {
        let engine = MonteCarloEngine::new(42);
        let scenarios = (0..=MAX_SCENARIOS)
            .map(|i| Scenario {
                label: format!("scenario_{i}"),
                overrides: ParamOverrides::default(),
            })
            .collect();
        let err = sim(&engine, &sim_request_with(scenarios)).unwrap_err();
        assert!(format!("{err}").contains("trop de scénarios"));
    }

    #[test]
    fn forecast_zero_growth_returns_constant_series() {
        let engine = MonteCarloEngine::new(42);
        let mut req = sim_request_with(vec![]);
        req.forecast = Some(ForecastConfig {
            months: 12,
            monthly_growth_pct: 0.0,
            base_volume_per_day: 10.0,
        });
        let res = sim(&engine, &req).unwrap();
        let f = res.forecast.unwrap();
        assert_eq!(f.baseline_monthly_co2eq_g.len(), 12);
        let first = f.baseline_monthly_co2eq_g[0];
        for v in &f.baseline_monthly_co2eq_g {
            assert!(
                (v - first).abs() < 1e-9,
                "série pas constante : {v} ≠ {first}"
            );
        }
        // Annual total = 12 × month 0.
        assert!((f.baseline_annual_co2eq_g - first * 12.0).abs() < 1e-6);
    }

    #[test]
    fn forecast_positive_growth_produces_geometric_series() {
        let engine = MonteCarloEngine::new(42);
        let mut req = sim_request_with(vec![]);
        req.forecast = Some(ForecastConfig {
            months: 12,
            monthly_growth_pct: 5.0,
            base_volume_per_day: 10.0,
        });
        let res = sim(&engine, &req).unwrap();
        let f = res.forecast.unwrap();
        // month 11 = month 0 × 1.05^11
        let expected_last = f.baseline_monthly_co2eq_g[0] * 1.05_f64.powi(11);
        let last = *f.baseline_monthly_co2eq_g.last().unwrap();
        assert!(
            (last - expected_last).abs() < 1e-6,
            "geo series : last={last}, expected={expected_last}"
        );
    }

    #[test]
    fn forecast_invalid_months_rejected() {
        let engine = MonteCarloEngine::new(42);
        let mut req = sim_request_with(vec![]);
        req.forecast = Some(ForecastConfig {
            months: 0,
            monthly_growth_pct: 5.0,
            base_volume_per_day: 10.0,
        });
        assert!(sim(&engine, &req).is_err());
        req.forecast = Some(ForecastConfig {
            months: MAX_FORECAST_MONTHS + 1,
            monthly_growth_pct: 5.0,
            base_volume_per_day: 10.0,
        });
        assert!(sim(&engine, &req).is_err());
    }

    #[test]
    fn forecast_invalid_growth_rejected() {
        let engine = MonteCarloEngine::new(42);
        let mut req = sim_request_with(vec![]);
        req.forecast = Some(ForecastConfig {
            months: 12,
            monthly_growth_pct: 200.0,
            base_volume_per_day: 10.0,
        });
        assert!(sim(&engine, &req).is_err());
    }

    #[test]
    fn param_overrides_any_returns_true_when_any_field_set() {
        let mut o = ParamOverrides::default();
        assert!(!o.any());
        o.pue = Some(1.2);
        assert!(o.any());
    }

    #[test]
    fn empty_scenario_outcome_equals_baseline_within_seed_determinism() {
        // A scenario with empty overrides must reproduce the baseline
        // (same seed, same computation → same exact result).
        let engine = MonteCarloEngine::new(42);
        let scenarios = vec![Scenario {
            label: "no-op".into(),
            overrides: ParamOverrides::default(),
        }];
        let res = sim(&engine, &sim_request_with(scenarios)).unwrap();
        let baseline_p50 = res.baseline.indicators[0].interval.p50;
        let scenario_p50 = res.scenarios[0].result.indicators[0].interval.p50;
        assert!(
            (baseline_p50 - scenario_p50).abs() < 1e-9,
            "no-op scenario doit reproduire baseline (déterministe)"
        );
        assert!(res.scenarios[0].delta_co2eq_g.abs() < 1e-9);
        assert!(res.scenarios[0].delta_pct.abs() < 1e-9);
    }
}
