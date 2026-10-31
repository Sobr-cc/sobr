//! 12-month forecast (M16 / C15) and « Et si...? » simulation (M13 / C11).

use chrono::Utc;
use sobria_estimator::EstimationParams;
use tracing::info;

use super::{append_to_ledger, apply_datacenter_override, ensure_known_model, user_default_method};
use crate::{
    dto::{
        SimulationRequestDto, SimulationResultDto, YearlyForecastRequestDto,
        YearlyForecastResultDto,
    },
    error::{AppError, IpcResult},
    state::AppState,
};

/// Runs a 12-month forecast (M16 / C15) with a P5/P50/P95
/// uncertainty band and overlaid growth scenarios.
///
/// Steps:
/// 1. Validate the baseline model.
/// 2. `sobria_estimator::forecast_yearly()` (1 Monte-Carlo + deterministic projections).
/// 3. Log the baseline in the ledger (1 entry).
/// 4. Convert to the DTO.
pub fn forecast_yearly_budget(
    req: YearlyForecastRequestDto,
    state: &AppState,
) -> IpcResult<YearlyForecastResultDto> {
    ensure_known_model(&req.baseline.model_id)?;
    let core_req = req.into_core(Utc::now());
    // The forecaster honours the user's default methodology.
    let engine = sobria_estimator::engine_for(user_default_method(state));
    let result =
        sobria_estimator::forecast_yearly(engine.as_ref(), &core_req).map_err(AppError::from)?;

    // Baseline logging: rerun the baseline estimation to get a full
    // EstimationResult (with bins) to log.
    // forecast_yearly() could have returned it, but we keep the API
    // simple; the cost is low (1 extra Monte-Carlo run).
    let params =
        EstimationParams::for_model(&core_req.baseline.model_id).map_err(AppError::from)?;
    let baseline_full = engine
        .estimate(&core_req.baseline, &params)
        .map_err(AppError::from)?;
    let audit_id = append_to_ledger(state, &baseline_full)?;

    info!(
        baseline_model = %core_req.baseline.model_id,
        scenarios = result.scenarios.len(),
        months = core_req.months,
        audit_id,
        "forecast_yearly_budget: ok"
    );
    Ok(YearlyForecastResultDto::from_result(&result, audit_id))
}

/// Runs a « Et si...? » simulation (M13 / C11).
///
/// Steps:
/// 1. Validate the baseline model.
/// 2. Convert DTO → internal Rust types (with the baseline timestamp and the
///    datacenter override).
/// 3. `sobria_estimator::simulate(...)`.
/// 4. Log **only the baseline** in the ledger (see brief C11 §3).
/// 5. Return the full DTO (baseline with audit_id, scenarios without).
pub fn simulate_scenarios(
    req: SimulationRequestDto,
    state: &AppState,
) -> IpcResult<SimulationResultDto> {
    ensure_known_model(&req.baseline.model_id)?;
    // C25-A8 — Honours the baseline's `datacenter_id` by overriding
    // PUE/IF/WUE in the params before handing over to the estimator. The
    // signature of `sobria_estimator::simulate` now takes `baseline_params`
    // explicitly for this wiring.
    let mut params = EstimationParams::for_model(&req.baseline.model_id).map_err(AppError::from)?;
    apply_datacenter_override(&mut params, req.baseline.datacenter_id.as_deref())?;
    let sim_core = req.into_core(Utc::now());
    // The simulator honours the user's default methodology.
    let engine = sobria_estimator::engine_for(user_default_method(state));
    let result =
        sobria_estimator::simulate(engine.as_ref(), &sim_core, &params).map_err(AppError::from)?;

    // Log ONLY the baseline (the scenarios are exploratory).
    let audit_id = append_to_ledger(state, &result.baseline)?;

    info!(
        baseline_model = %result.baseline.request.model_id,
        scenarios = result.scenarios.len(),
        forecast = result.forecast.is_some(),
        audit_id,
        "simulate_scenarios: ok"
    );
    Ok(SimulationResultDto::from_result(&result, audit_id))
}
