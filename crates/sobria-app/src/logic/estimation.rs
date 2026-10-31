//! Prompt estimation: logged in the audit ledger, or ephemeral for comparisons.

use chrono::Utc;
use sobria_estimator::EstimationParams;
use tracing::{debug, info, warn};

use super::{
    append_to_ledger, apply_datacenter_override, user_default_method, validate_estimation_request,
};
use crate::{
    dto::{EstimationRequestDto, EstimationResultDto},
    error::{AppError, IpcResult},
    state::AppState,
};

/// Estimates a prompt and logs it in the ledger.
///
/// Steps:
/// 1. Validate the request.
/// 2. Look up `EstimationParams` for the `model_id`, with the datacenter
///    override, and save the datacenter as the last one used.
/// 3. Estimate with `engine_for(method)`: the explicit method, else the
///    user preference, else the default.
/// 4. Append to the ledger (lock held).
/// 5. Return the DTO + `audit_id`.
pub fn estimate_prompt(
    req: EstimationRequestDto,
    state: &AppState,
) -> IpcResult<EstimationResultDto> {
    // Model first (stable error for the UI), then a basic token check
    // before Monte-Carlo (the engine validates too).
    validate_estimation_request(&req)?;

    let model_id = req.model_id.clone();
    let method_override = req.method;
    let datacenter_id = req.datacenter_id.clone();
    let core_req = req.into_core(Utc::now());
    let mut params = EstimationParams::for_model(&model_id).map_err(AppError::from)?;
    apply_datacenter_override(&mut params, datacenter_id.as_deref())?;

    // Methodology choice: explicit override > user preference > default.
    let method = method_override.unwrap_or_else(|| user_default_method(state));
    let engine = sobria_estimator::engine_for(method);

    let result = engine
        .estimate(&core_req, &params)
        .map_err(AppError::from)?;

    let audit_id = append_to_ledger(state, &result)?;

    // Persist "last used" (C25). Best-effort: a failure does not break
    // the estimation; we log and carry on. The current value (Some or None)
    // is saved, symmetrically, so the user can go back to
    // "none selected" and that persists too.
    if let Ok(mut store) = state.preferences.lock() {
        if let Err(e) = store.set_default_datacenter_id(datacenter_id.as_deref()) {
            warn!(error = ?e, "set_default_datacenter_id échoué (non-bloquant)");
        }
    }

    info!(
        model = %model_id,
        audit_id,
        co2eq_p50 = ?result.indicators.first().map(|i| i.interval.p50),
        "estimate_prompt: ok"
    );
    Ok(EstimationResultDto::from_result(&result, audit_id))
}

/// Runs an **ephemeral** estimation for the "Voir aussi" panel (C24).
///
/// Differences from [`estimate_prompt`]:
/// - The methodology is **always** set by the caller (no
///   fallback to the user preference: this is explicitly a
///   comparison).
/// - The result is **NOT logged** in the audit ledger.
///
/// Why not log? The "Voir aussi" panel runs 1 to N
/// alternative methodologies for each main estimation, which
/// would multiply the ledger volume with no added value for
/// the user: these computations are *exploratory*, not decisions.
/// The main estimation (the default methodology) does stay in the
/// ledger via [`estimate_prompt`]; it is the one used for
/// CSRD reports, the dashboard and the Gebru datasheet.
///
/// The returned DTO's `audit_id` is `0` (sentinel for "not logged").
///
/// This function is *stateless* (no access to `AppState`): a
/// comparison has no reason to read user preferences or touch the
/// ledger. This is deliberate.
pub fn estimate_for_comparison(
    req: EstimationRequestDto,
    method: sobria_core::EmpreinteMethod,
) -> IpcResult<EstimationResultDto> {
    validate_estimation_request(&req)?;

    let model_id = req.model_id.clone();
    let datacenter_id = req.datacenter_id.clone();
    let core_req = req.into_core(Utc::now());
    let mut params = EstimationParams::for_model(&model_id).map_err(AppError::from)?;
    // C25-A6 — applies the datacenter override (PUE/IF/WUE) before Monte-Carlo.
    // Stateless: no prefs persistence here, unlike
    // `estimate_prompt`; this is deliberate (see the rustdoc above).
    apply_datacenter_override(&mut params, datacenter_id.as_deref())?;
    let engine = sobria_estimator::engine_for(method);
    let result = engine
        .estimate(&core_req, &params)
        .map_err(AppError::from)?;

    debug!(
        model = %model_id,
        method = method.as_str(),
        co2eq_p50 = ?result.indicators.first().map(|i| i.interval.p50),
        "estimate_for_comparison: ok (no ledger write)"
    );
    // audit_id = 0 → sentinel for "ephemeral estimation, not logged".
    Ok(EstimationResultDto::from_result(&result, 0))
}
