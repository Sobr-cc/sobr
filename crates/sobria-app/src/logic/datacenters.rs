//! Datacenters (C12 — M12).

use chrono::Utc;
use sobria_core::{EstimationRequest, Indicator};
use sobria_estimator::{Distribution, EstimationParams};
use sobria_geoloc::{aggregate_by_country, all_datacenters, find_datacenter, DatacenterRecord};

use super::{pick_p50, user_default_method};
use crate::{
    dto::{CountryAggregateDto, DatacenterDetailDto, DatacenterSummaryDto},
    error::{AppError, IpcError, IpcResult},
    state::AppState,
};

/// Lists all known European datacenters (28 in v1.0).
pub fn list_datacenters() -> IpcResult<Vec<DatacenterSummaryDto>> {
    Ok(all_datacenters().iter().map(Into::into).collect())
}

/// Returns a datacenter's details with a computed baseline (gpt-4o-mini,
/// 100/500 tokens) for the bars of the drill-down UI.
pub fn get_datacenter_detail(id: &str, state: &AppState) -> IpcResult<DatacenterDetailDto> {
    let dc = find_datacenter(id).ok_or_else(|| {
        IpcError::new("not_found", format!("datacenter '{id}' inconnu"))
            .with_details(serde_json::json!({ "id": id }))
    })?;
    let (co2, energy, water) = compute_baseline_for_dc(dc, state)?;
    Ok(DatacenterDetailDto {
        id: dc.id.clone(),
        name: dc.name.clone(),
        operator: dc.operator.clone(),
        country_iso: dc.country_iso.clone(),
        city: dc.city.clone(),
        lat: dc.lat,
        lon: dc.lon,
        pue: dc.pue,
        if_electrical_g_per_kwh: dc.if_electrical_g_per_kwh,
        wue_l_per_kwh: dc.wue_l_per_kwh,
        capacity_mw: dc.capacity_mw,
        sources: dc.sources.clone(),
        hourly_profile_24h: dc.hourly_profile_24h.clone(),
        baseline_co2eq_p50_g: co2,
        baseline_energy_wh_p50: energy,
        baseline_water_l_p50: water,
    })
}

/// Aggregates the 28 datacenters by country (13 countries in v1.0).
pub fn aggregate_datacenters_by_country() -> IpcResult<Vec<CountryAggregateDto>> {
    Ok(aggregate_by_country().iter().map(Into::into).collect())
}

/// Computes a baseline (CO₂eq, energy, water) for a given DC with a
/// gpt-4o-mini 100/500-token reference prompt. The DC's PUE/IF/WUE params
/// are injected as `Distribution::Point` (deterministic).
///
/// **Cross-DC determinism**: so that DC-to-DC comparisons stay
/// numerically comparable (same Monte-Carlo seed, same draws),
/// every "scalar" parameter is forced to `Point`. For DCs that
/// do not publish their WUE, a median value from the literature is used
/// (1.5 L/kWh — Mytton 2021), so that a `Uniform` distribution does not
/// shift the RNG state between two DCs and hide the difference
/// in electricity mix. This default WUE does not show up in the returned
/// DTO (`wue_l_per_kwh` stays `None` on the frontend).
fn compute_baseline_for_dc(
    dc: &DatacenterRecord,
    state: &AppState,
) -> Result<(f64, f64, f64), AppError> {
    const REF_MODEL: &str = "gpt-4o-mini";
    const REF_TOKENS_IN: u32 = 100;
    const REF_TOKENS_OUT: u32 = 500;
    /// Mytton 2021 median, used only for DCs that do not publish
    /// their WUE. Keeps the RNG deterministic across DCs.
    const WUE_DEFAULT_L_PER_KWH: f64 = 1.5;

    let mut params = EstimationParams::for_model(REF_MODEL)?;
    params.pue = Distribution::Point { value: dc.pue };
    params.if_electrical_g_per_kwh = Distribution::Point {
        value: dc.if_electrical_g_per_kwh,
    };
    params.wue_l_per_kwh = Distribution::Point {
        value: dc.wue_l_per_kwh.unwrap_or(WUE_DEFAULT_L_PER_KWH),
    };
    params.validate()?;

    let req = EstimationRequest {
        model_id: REF_MODEL.into(),
        tokens_in: REF_TOKENS_IN,
        tokens_out_estimated: REF_TOKENS_OUT,
        datacenter_id: Some(dc.id.clone()),
        timestamp: Utc::now(),
        modalities: Vec::new(),
        overhead: sobria_core::ContextOverhead::default(),
    };
    // The DC baseline honours the user's methodology.
    let engine = sobria_estimator::engine_for(user_default_method(state));
    let result = engine.estimate(&req, &params)?;
    let co2 = pick_p50(&result, Indicator::Co2Eq);
    let energy = pick_p50(&result, Indicator::Energy);
    let water = pick_p50(&result, Indicator::Water);
    Ok((co2, energy, water))
}
