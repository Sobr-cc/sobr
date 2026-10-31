//! Model catalogue: runtime metadata, model presets, vendor comparison,
//! methodologies and model detail sheets.

use chrono::Utc;
use sobria_core::{EstimationRequest, Indicator};
use sobria_estimator::{available_models, find_preset, EstimationParams};

use super::{user_default_method, APP_VERSION};
use crate::{
    dashboard,
    dto::{
        architecture_str, calibration_str, model_family_str, openness_str, MetaInfo,
        ModelDetailDto, ModelPresetDto,
    },
    error::{AppError, IpcError, IpcResult},
    state::AppState,
};

/// Runtime metadata.
pub fn meta_info(state: &AppState) -> IpcResult<MetaInfo> {
    Ok(MetaInfo {
        app_version: APP_VERSION.into(),
        estimator_seed: state.estimator.seed(),
        estimator_n: state.estimator.n(),
        audit_path: state.audit_path.display().to_string(),
        data_root: state.data_root.display().to_string(),
    })
}

/// Lists the available model presets (≥ 8, see C06).
pub fn list_models() -> IpcResult<Vec<ModelPresetDto>> {
    Ok(available_models().into_iter().map(Into::into).collect())
}

/// **C32.4** — Aggregates vendor disclosures by vendor for the M9
/// comparison table. Returns one row per vendor present in
/// `MODEL_REGISTRY`, with two booleans (training / prompt-level) that
/// let the UI show a ✅ or a ❌ « Pas de disclosure ».
///
/// Source of truth: `sobria_estimator::MODEL_REGISTRY[*].vendor_disclosures`.
pub fn list_vendor_comparison() -> IpcResult<Vec<crate::dto::VendorComparisonRowDto>> {
    use sobria_estimator::{VendorScope, MODEL_REGISTRY};
    use std::collections::BTreeMap;

    // BTreeMap: stable alphabetical order for the UI, no JS sort needed.
    let mut by_vendor: BTreeMap<String, (bool, bool, Option<String>)> = BTreeMap::new();

    for preset in MODEL_REGISTRY {
        let entry = by_vendor
            .entry(preset.provider.to_string())
            .or_insert((false, false, None));
        for d in preset.vendor_disclosures {
            match d.scope {
                VendorScope::Training => entry.1 = true,
                VendorScope::InferencePerPrompt => entry.0 = true,
            }
            if entry.2.is_none() {
                entry.2 = Some(d.source_url.to_string());
            }
        }
    }

    Ok(by_vendor
        .into_iter()
        .map(
            |(vendor, (has_prompt_level, has_training, primary_source_url))| {
                crate::dto::VendorComparisonRowDto {
                    vendor,
                    has_prompt_level,
                    has_training,
                    primary_source_url,
                }
            },
        )
        .collect())
}

/// Lists the LLM footprint methodologies built into Sobr.ia (C24).
///
/// This IPC command feeds the `/methodologies` page (catalogue) and
/// the Settings → "Méthodologie par défaut" / "Voir aussi" selectors.
/// No side effects: read-only access to the compile-time registry.
pub fn list_methodologies() -> IpcResult<Vec<crate::dto::MethodologyInfoDto>> {
    Ok(sobria_estimator::AVAILABLE_METHODS
        .iter()
        .map(crate::dto::MethodologyInfoDto::from)
        .collect())
}

/// Details a model (M9 / C18) with its distribution params and a
/// context baseline computed on a 100/500-token reference prompt.
///
/// **Not logged** in the audit ledger: this is a static, educational
/// sheet, not a user estimation.
#[allow(clippy::too_many_lines)] // C34.5 — 16 capability fields; helpers would hurt readability.
pub fn get_model_detail(model_id: &str, state: &AppState) -> IpcResult<ModelDetailDto> {
    let preset = find_preset(model_id)
        .ok_or_else(|| IpcError::new("not_found", format!("modèle '{model_id}' inconnu")))?;

    // Baseline for context (not logged).
    // Honours the user's default methodology; otherwise the model sheet
    // would show an AFNOR baseline while the user computes with EcoLogits
    // (misleading display).
    let params = EstimationParams::for_model(model_id).map_err(AppError::from)?;
    let req = EstimationRequest {
        model_id: model_id.to_string(),
        tokens_in: 100,
        tokens_out_estimated: 500,
        datacenter_id: None,
        timestamp: Utc::now(),
        modalities: Vec::new(),
        overhead: sobria_core::ContextOverhead::default(),
    };
    let engine = sobria_estimator::engine_for(user_default_method(state));
    let result = engine.estimate(&req, &params).map_err(AppError::from)?;
    let co2 = result
        .indicators
        .iter()
        .find(|i| matches!(i.indicator, Indicator::Co2Eq))
        .ok_or_else(|| AppError::Internal("CO2eq indicator manquant".into()))?;
    let energy_p50 = dashboard::indicator_p50(&result.indicators, Indicator::Energy).unwrap_or(0.0);
    let water_p50 = dashboard::indicator_p50(&result.indicators, Indicator::Water).unwrap_or(0.0);

    Ok(ModelDetailDto {
        id: preset.id.into(),
        display_name: preset.display_name.into(),
        provider: preset.provider.into(),
        family: preset.family.into(),
        approx_params_billions: preset.approx_params_billions,
        openness: openness_str(preset.openness).into(),
        calibration: calibration_str(preset.calibration).into(),
        sources: preset.sources.iter().map(|s| (*s).to_string()).collect(),
        epsilon_prefill_mj_per_token: preset.epsilon_prefill_mj.into(),
        epsilon_decode_mj_per_token: preset.epsilon_decode_mj.into(),
        embodied_g_per_request: preset.embodied_g_per_req.into(),
        baseline_co2eq_p5_g: co2.interval.p5,
        baseline_co2eq_p50_g: co2.interval.p50,
        baseline_co2eq_p95_g: co2.interval.p95,
        baseline_energy_wh_p50: energy_p50,
        baseline_water_l_p50: water_p50,
        // C32.4 — official vendor disclosures.
        vendor_disclosures: preset.vendor_disclosures.iter().map(Into::into).collect(),
        // C34.5 — model capabilities for the M9 sheet.
        release_date: preset.release_date.into(),
        active_params_b: preset.active_params_b,
        model_family: model_family_str(preset.model_family).into(),
        architecture: architecture_str(preset.architecture).into(),
        moe_experts: match preset.architecture {
            sobria_estimator::ArchitectureKind::Moe { experts, .. } => Some(experts),
            _ => None,
        },
        moe_active_experts: match preset.architecture {
            sobria_estimator::ArchitectureKind::Moe { active_experts, .. } => Some(active_experts),
            _ => None,
        },
        vision_capable: preset.vision_capable,
        vision_pricing: preset.vision_pricing,
        audio_capable: preset.audio_capable,
        reasoning_capable: preset.reasoning_capable,
        thinking_token_multiplier: preset.thinking_token_multiplier,
        default_context_overhead_tokens: preset.default_context_overhead_tokens,
        deprecated: preset.deprecated,
        source_url: preset.source_url.into(),
    })
}
