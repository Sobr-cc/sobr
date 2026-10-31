//! Forecaster over 1 to `MAX_FORECAST_HORIZON_MONTHS` months — module M16 (chantier C15).
//!
//! Projects the cumulative footprint of an AI usage with a **P5/P50/P95
//! uncertainty band** propagated month by month, and overlays several
//! adoption growth scenarios.
//!
//! See `docs/archive/chantiers/C15-forecaster-12-mois.md`.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sobria_core::{EstimationRequest, Indicator};

use crate::{
    engine_trait::find_indicator,
    error::{EstimatorError, EstimatorResult},
    forecast,
    params::EstimationParams,
};

/// Maximum number of scenarios per forecast request.
pub const MAX_FORECAST_SCENARIOS: usize = 10;

/// Maximum horizon of a projection (months).
pub const MAX_FORECAST_HORIZON_MONTHS: u32 = 60;

/// Configuration of a monthly adoption scenario.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct YearlyScenario {
    /// Label shown by the UI (e.g. « Status quo », « +10% / mois »).
    pub label: String,
    /// Monthly growth in percent (e.g. 5.0 = +5%/month).
    /// Negative values allowed (usage decline).
    pub monthly_growth_pct: f64,
}

/// Forecast request.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct YearlyForecastRequest {
    /// Baseline estimation — used to extract `co2eq_p5/p50/p95`.
    pub baseline: EstimationRequest,
    /// Scenarios to project (1..=`MAX_FORECAST_SCENARIOS`).
    pub scenarios: Vec<YearlyScenario>,
    /// Projection horizon in months.
    pub months: u32,
    /// Baseline volume (prompts per day at month 0).
    pub base_volume_per_day: f64,
}

/// Result of one scenario: monthly + cumulative series + annual total.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct YearlyScenarioOutcome {
    pub label: String,
    pub monthly_growth_pct: f64,
    pub monthly_p5_g: Vec<f64>,
    pub monthly_p50_g: Vec<f64>,
    pub monthly_p95_g: Vec<f64>,
    pub cumulative_p5_g: Vec<f64>,
    pub cumulative_p50_g: Vec<f64>,
    pub cumulative_p95_g: Vec<f64>,
    /// Totals over `months`, not a calendar year despite the name (SOB-210).
    pub annual_p5_g: f64,
    pub annual_p50_g: f64,
    pub annual_p95_g: f64,
}

/// Overall forecast result.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct YearlyForecastResult {
    /// Baseline quantiles (1 single request, no volume aggregation).
    pub baseline_co2eq_p5_g: f64,
    pub baseline_co2eq_p50_g: f64,
    pub baseline_co2eq_p95_g: f64,
    pub scenarios: Vec<YearlyScenarioOutcome>,
}

// ─────────────────────────────────────────────────────────────────────────────
// forecast_yearly()
// ─────────────────────────────────────────────────────────────────────────────

/// Runs the forecast over 1-60 months for 1-10 growth scenarios.
///
/// **Errors**:
/// - `Schema` if `scenarios` is empty or > `MAX_FORECAST_SCENARIOS`.
/// - `Schema` if `months` is outside `[1, MAX_FORECAST_HORIZON_MONTHS]`.
/// - `Schema` if `base_volume_per_day` is outside `[0, 10⁶]`.
/// - `Schema` if a `monthly_growth_pct` is outside `[-50, 50]`.
/// - `Schema` if two scenarios share the same `label`.
/// - propagates the errors of `engine.estimate`.
pub fn forecast_yearly(
    engine: &dyn crate::engine_trait::EmpreinteEngine,
    req: &YearlyForecastRequest,
) -> EstimatorResult<YearlyForecastResult> {
    validate(req)?;

    let params = EstimationParams::for_model(&req.baseline.model_id)?;
    let result = engine.estimate(&req.baseline, &params)?;
    let co2 = find_indicator(&result, Indicator::Co2Eq)
        .ok_or_else(|| EstimatorError::Schema("CO2eq indicator manquant".into()))?;
    let p5 = co2.interval.p5;
    let p50 = co2.interval.p50;
    let p95 = co2.interval.p95;

    let mut outcomes = Vec::with_capacity(req.scenarios.len());
    for s in &req.scenarios {
        let outcome = project_scenario(
            &s.label,
            s.monthly_growth_pct,
            req.months,
            req.base_volume_per_day,
            p5,
            p50,
            p95,
        );
        outcomes.push(outcome);
    }

    Ok(YearlyForecastResult {
        baseline_co2eq_p5_g: p5,
        baseline_co2eq_p50_g: p50,
        baseline_co2eq_p95_g: p95,
        scenarios: outcomes,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// helpers
// ─────────────────────────────────────────────────────────────────────────────

fn validate(req: &YearlyForecastRequest) -> EstimatorResult<()> {
    if req.scenarios.is_empty() {
        return Err(EstimatorError::Schema(
            "au moins un scénario de forecast requis".into(),
        ));
    }
    forecast::check_scenario_count(req.scenarios.len(), MAX_FORECAST_SCENARIOS)?;
    forecast::check_months(req.months, MAX_FORECAST_HORIZON_MONTHS)?;
    forecast::check_volume_per_day(req.base_volume_per_day)?;
    let mut seen_labels = std::collections::HashSet::new();
    for s in &req.scenarios {
        if !forecast::growth_in_bounds(s.monthly_growth_pct) {
            return Err(EstimatorError::Schema(format!(
                "monthly_growth_pct {} hors bornes [-50, 50] pour '{}'",
                s.monthly_growth_pct, s.label
            )));
        }
        if !seen_labels.insert(s.label.as_str()) {
            return Err(forecast::duplicate_label(&s.label));
        }
    }
    Ok(())
}

#[allow(clippy::similar_names)] // p5 / p50 / p95 are intentional and readable
fn project_scenario(
    label: &str,
    growth_pct: f64,
    months: u32,
    volume_per_day: f64,
    baseline_p5: f64,
    baseline_p50: f64,
    baseline_p95: f64,
) -> YearlyScenarioOutcome {
    let growth = forecast::growth_factor(growth_pct);
    let base_factor = volume_per_day * forecast::DAYS_PER_MONTH;
    let factors: Vec<f64> = (0..months)
        .map(|n| forecast::grown(base_factor, growth, n))
        .collect();
    let (monthly_p5_g, cumulative_p5_g, annual_p5_g) = project(baseline_p5, &factors);
    let (monthly_p50_g, cumulative_p50_g, annual_p50_g) = project(baseline_p50, &factors);
    let (monthly_p95_g, cumulative_p95_g, annual_p95_g) = project(baseline_p95, &factors);
    YearlyScenarioOutcome {
        label: label.to_string(),
        monthly_growth_pct: growth_pct,
        monthly_p5_g,
        monthly_p50_g,
        monthly_p95_g,
        cumulative_p5_g,
        cumulative_p50_g,
        cumulative_p95_g,
        annual_p5_g,
        annual_p50_g,
        annual_p95_g,
    }
}

/// Monthly values, running totals and total of one quantile.
fn project(baseline: f64, factors: &[f64]) -> (Vec<f64>, Vec<f64>, f64) {
    let monthly: Vec<f64> = factors.iter().map(|factor| baseline * factor).collect();
    let mut total = 0.0_f64;
    let cumulative = monthly
        .iter()
        .map(|m| {
            total += m;
            total
        })
        .collect();
    (monthly, cumulative, total)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::MonteCarloEngine;

    fn baseline_request() -> EstimationRequest {
        crate::validation::request_for("gpt-4o-mini", 100, 500)
    }

    fn simple_request(scenarios: Vec<YearlyScenario>) -> YearlyForecastRequest {
        YearlyForecastRequest {
            baseline: baseline_request(),
            scenarios,
            months: 12,
            base_volume_per_day: 100.0,
        }
    }

    #[test]
    fn empty_scenarios_rejected() {
        let engine = MonteCarloEngine::default();
        let req = simple_request(vec![]);
        let err = forecast_yearly(&engine, &req).unwrap_err();
        assert!(format!("{err}").contains("au moins un scénario"));
    }

    #[test]
    fn too_many_scenarios_rejected() {
        let engine = MonteCarloEngine::default();
        let scenarios: Vec<YearlyScenario> = (0..=MAX_FORECAST_SCENARIOS)
            .map(|i| YearlyScenario {
                label: format!("sc{i}"),
                monthly_growth_pct: 0.0,
            })
            .collect();
        let req = simple_request(scenarios);
        let err = forecast_yearly(&engine, &req).unwrap_err();
        assert!(format!("{err}").contains("trop de scénarios"));
    }

    #[test]
    fn invalid_months_rejected() {
        let engine = MonteCarloEngine::default();
        let mut req = simple_request(vec![YearlyScenario {
            label: "ok".into(),
            monthly_growth_pct: 0.0,
        }]);
        req.months = 0;
        assert!(forecast_yearly(&engine, &req).is_err());
        req.months = MAX_FORECAST_HORIZON_MONTHS + 1;
        assert!(forecast_yearly(&engine, &req).is_err());
    }

    #[test]
    fn invalid_growth_rejected() {
        let engine = MonteCarloEngine::default();
        let req = simple_request(vec![YearlyScenario {
            label: "fou".into(),
            monthly_growth_pct: 200.0,
        }]);
        assert!(forecast_yearly(&engine, &req).is_err());
    }

    #[test]
    fn duplicate_labels_rejected() {
        let engine = MonteCarloEngine::default();
        let req = simple_request(vec![
            YearlyScenario {
                label: "x".into(),
                monthly_growth_pct: 0.0,
            },
            YearlyScenario {
                label: "x".into(),
                monthly_growth_pct: 5.0,
            },
        ]);
        let err = forecast_yearly(&engine, &req).unwrap_err();
        assert!(format!("{err}").contains("doublon"));
    }

    #[test]
    fn unknown_model_rejected() {
        let engine = MonteCarloEngine::default();
        let mut req = simple_request(vec![YearlyScenario {
            label: "ok".into(),
            monthly_growth_pct: 0.0,
        }]);
        req.baseline.model_id = "does-not-exist".into();
        assert!(forecast_yearly(&engine, &req).is_err());
    }

    #[test]
    fn growth_zero_produces_constant_series() {
        let engine = MonteCarloEngine::new(42);
        let req = simple_request(vec![YearlyScenario {
            label: "status-quo".into(),
            monthly_growth_pct: 0.0,
        }]);
        let res = forecast_yearly(&engine, &req).unwrap();
        let outcome = &res.scenarios[0];
        assert_eq!(outcome.monthly_p50_g.len(), 12);
        let first = outcome.monthly_p50_g[0];
        for v in &outcome.monthly_p50_g {
            assert!((v - first).abs() < 1e-9, "série pas constante");
        }
    }

    #[test]
    fn growth_5pct_produces_geometric_series() {
        let engine = MonteCarloEngine::new(42);
        let req = simple_request(vec![YearlyScenario {
            label: "growth-5".into(),
            monthly_growth_pct: 5.0,
        }]);
        let res = forecast_yearly(&engine, &req).unwrap();
        let outcome = &res.scenarios[0];
        let m0 = outcome.monthly_p50_g[0];
        let m11 = outcome.monthly_p50_g[11];
        let expected = m0 * 1.05_f64.powi(11);
        assert!(
            (m11 - expected).abs() < 1e-6,
            "geom progress: m11={m11}, expected={expected}"
        );
    }

    #[test]
    fn cumulative_is_strictly_increasing_with_positive_growth() {
        let engine = MonteCarloEngine::new(42);
        let req = simple_request(vec![YearlyScenario {
            label: "growth-5".into(),
            monthly_growth_pct: 5.0,
        }]);
        let res = forecast_yearly(&engine, &req).unwrap();
        let cum = &res.scenarios[0].cumulative_p50_g;
        for i in 1..cum.len() {
            assert!(
                cum[i] > cum[i - 1],
                "cumul doit croître strictement (i={i}, prev={}, cur={})",
                cum[i - 1],
                cum[i]
            );
        }
        // annual_p50_g == last cumulative value
        assert!((res.scenarios[0].annual_p50_g - cum[cum.len() - 1]).abs() < 1e-9);
    }

    #[test]
    fn p5_p50_p95_ordering_preserved_each_month() {
        let engine = MonteCarloEngine::new(42);
        let req = simple_request(vec![YearlyScenario {
            label: "g".into(),
            monthly_growth_pct: 5.0,
        }]);
        let res = forecast_yearly(&engine, &req).unwrap();
        let o = &res.scenarios[0];
        for n in 0..(o.monthly_p5_g.len()) {
            assert!(
                o.monthly_p5_g[n] <= o.monthly_p50_g[n] + 1e-9,
                "P5 ≤ P50 violé au mois {n}"
            );
            assert!(
                o.monthly_p50_g[n] <= o.monthly_p95_g[n] + 1e-9,
                "P50 ≤ P95 violé au mois {n}"
            );
            assert!(
                o.cumulative_p5_g[n] <= o.cumulative_p50_g[n] + 1e-9,
                "cum P5 ≤ P50 violé au mois {n}"
            );
        }
    }

    #[test]
    fn multi_scenarios_overlay_all_present() {
        let engine = MonteCarloEngine::new(42);
        let req = simple_request(vec![
            YearlyScenario {
                label: "status-quo".into(),
                monthly_growth_pct: 0.0,
            },
            YearlyScenario {
                label: "acceleration".into(),
                monthly_growth_pct: 10.0,
            },
            YearlyScenario {
                label: "ralentissement".into(),
                monthly_growth_pct: -5.0,
            },
        ]);
        let res = forecast_yearly(&engine, &req).unwrap();
        assert_eq!(res.scenarios.len(), 3);
        // Acceleration exceeds status quo, slowdown stays below.
        let sq = res.scenarios[0].annual_p50_g;
        let acc = res.scenarios[1].annual_p50_g;
        let ralenti = res.scenarios[2].annual_p50_g;
        assert!(acc > sq, "acc {acc} doit dépasser sq {sq}");
        assert!(ralenti < sq, "ralenti {ralenti} doit être < sq {sq}");
    }

    #[test]
    fn zero_volume_produces_zero_series() {
        let engine = MonteCarloEngine::new(42);
        let mut req = simple_request(vec![YearlyScenario {
            label: "ok".into(),
            monthly_growth_pct: 5.0,
        }]);
        req.base_volume_per_day = 0.0;
        let res = forecast_yearly(&engine, &req).unwrap();
        for v in &res.scenarios[0].monthly_p50_g {
            assert!((*v).abs() < 1e-12);
        }
        assert!(res.scenarios[0].annual_p50_g.abs() < 1e-12);
    }
}
