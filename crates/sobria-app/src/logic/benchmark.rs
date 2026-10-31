//! Model benchmark (M3 / C17): one prompt, N models, rankings by P50.

use sobria_estimator::find_preset;
use tracing::info;

use super::{estimate_prompt, pick_p50_from_dto, user_default_method};
use crate::{
    dto::{
        calibration_str, openness_str, BenchmarkOutcomeDto, BenchmarkRequestDto,
        BenchmarkResultDto, EstimationRequestDto,
    },
    error::{AppError, IpcError, IpcResult},
    state::AppState,
};

/// Upper bound on the number of models allowed in a benchmark.
pub const MAX_BENCHMARK_MODELS: usize = 20;

/// Compares N models (1..=`MAX_BENCHMARK_MODELS`) on the same prompt (M3 / C17).
///
/// Simply loops over `estimate_prompt`, then computes rankings
/// by ascending P50 for CO₂eq / energy / water. Each call logs
/// to the audit ledger (1 entry per model).
#[allow(clippy::too_many_lines)] // linear flow: validations + N estimations + 3 rankings
pub fn benchmark_models(
    req: BenchmarkRequestDto,
    state: &AppState,
) -> IpcResult<BenchmarkResultDto> {
    // 1. Validations.
    if req.model_ids.is_empty() {
        return Err(IpcError::invalid_request("model_ids vide"));
    }
    if req.model_ids.len() > MAX_BENCHMARK_MODELS {
        return Err(IpcError::invalid_request(format!(
            "trop de modèles : {} (max {MAX_BENCHMARK_MODELS})",
            req.model_ids.len()
        )));
    }
    let mut seen = std::collections::HashSet::new();
    for m in &req.model_ids {
        if !seen.insert(m.as_str()) {
            return Err(IpcError::invalid_request(format!(
                "model_id '{m}' en doublon"
            )));
        }
    }
    let mut unknown: Vec<&str> = Vec::new();
    for m in &req.model_ids {
        if find_preset(m).is_none() {
            unknown.push(m.as_str());
        }
    }
    if !unknown.is_empty() {
        return Err(IpcError::from(AppError::UnknownModel(unknown.join(", "))));
    }

    // 2. Estimate and log each model.
    //
    // Reads the default methodology **once** instead of letting each
    // `estimate_prompt` re-read it from SQLite. This:
    // 1. avoids N disk reads (1 per benchmarked model),
    // 2. guarantees that ALL models use the **same** methodology
    //    even if the user changes their preferences during the run
    //    (race-condition mitigation: a benchmark must be consistent).
    let user_method = user_default_method(state);

    let mut tmp: Vec<(BenchmarkOutcomeDto, f64, f64, f64)> =
        Vec::with_capacity(req.model_ids.len());
    for model_id in &req.model_ids {
        let est_req = EstimationRequestDto {
            model_id: model_id.clone(),
            tokens_in: req.tokens_in,
            tokens_out_estimated: req.tokens_out_estimated,
            datacenter_id: req.datacenter_id.clone(),
            // Explicit methodology: no IPC fallback, it is already resolved.
            method: Some(user_method),
            modalities: None,
            overhead: None,
        };
        let result_dto = estimate_prompt(est_req, state)?;
        let preset = find_preset(model_id)
            .ok_or_else(|| IpcError::from(AppError::UnknownModel(model_id.clone())))?;
        let co2_p50 = pick_p50_from_dto(&result_dto, "co2eq");
        let energy_p50 = pick_p50_from_dto(&result_dto, "energy");
        let water_p50 = pick_p50_from_dto(&result_dto, "water");
        let outcome = BenchmarkOutcomeDto {
            model_id: preset.id.into(),
            display_name: preset.display_name.into(),
            provider: preset.provider.into(),
            family: preset.family.into(),
            openness: openness_str(preset.openness).into(),
            calibration: calibration_str(preset.calibration).into(),
            result: result_dto,
            rank_co2eq: 0, // assigned below
            rank_energy: 0,
            rank_water: 0,
        };
        tmp.push((outcome, co2_p50, energy_p50, water_p50));
    }

    // 3. Ascending rankings (rank 1 = best = lowest P50).
    let (ranking_co2, rank_co2) = rank_ascending(&tmp, |v| v.1);
    let (ranking_energy, rank_energy) = rank_ascending(&tmp, |v| v.2);
    let (ranking_water, rank_water) = rank_ascending(&tmp, |v| v.3);

    // `tmp` is already in the user's requested order (useful for the UI).
    let outcomes: Vec<BenchmarkOutcomeDto> = tmp
        .into_iter()
        .enumerate()
        .map(|(i, (mut o, _, _, _))| {
            o.rank_co2eq = rank_co2[i];
            o.rank_energy = rank_energy[i];
            o.rank_water = rank_water[i];
            o
        })
        .collect();

    info!(
        models = req.model_ids.len(),
        winner_co2 = %ranking_co2.first().map_or("", String::as_str),
        "benchmark_models: ok"
    );

    Ok(BenchmarkResultDto {
        outcomes,
        ranking_by_co2eq_p50: ranking_co2,
        ranking_by_energy_p50: ranking_energy,
        ranking_by_water_p50: ranking_water,
        tokens_in: req.tokens_in,
        tokens_out_estimated: req.tokens_out_estimated,
    })
}

/// Model ids sorted by ascending `key` (stable, incomparable values tie),
/// and the 1-based rank of each item at its index in `items`.
fn rank_ascending<F>(
    items: &[(BenchmarkOutcomeDto, f64, f64, f64)],
    key: F,
) -> (Vec<String>, Vec<u32>)
where
    F: Fn(&(BenchmarkOutcomeDto, f64, f64, f64)) -> f64,
{
    let mut indices: Vec<usize> = (0..items.len()).collect();
    indices.sort_by(|a, b| {
        key(&items[*a])
            .partial_cmp(&key(&items[*b]))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let mut ranks = vec![0; items.len()];
    for (pos, &i) in indices.iter().enumerate() {
        // Bounded by MAX_BENCHMARK_MODELS (= 20) → safe cast.
        ranks[i] = u32::try_from(pos).unwrap_or(u32::MAX).saturating_add(1);
    }
    let ids = indices
        .into_iter()
        .map(|i| items[i].0.model_id.clone())
        .collect();
    (ids, ranks)
}
