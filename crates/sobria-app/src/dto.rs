//! Data Transfer Objects between Rust (Tauri) and the SvelteKit frontend.
//!
//! Conventions:
//! - All DTOs are `serde::Serialize` (IPC output).
//! - Input DTOs (`*Dto` on the request side) are also `Deserialize`.
//! - No `Option<T>` on input unless the UI field is truly optional (e.g. `datacenter_id`).
//! - Timestamps are RFC 3339 `String`s (JavaScript-friendly).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sobria_audit::{AuditEntry, IntegrityReport};
use sobria_core::{
    DistributionBins, Equivalent, EstimationRequest, EstimationResult, Hypothesis, Indicator,
    IndicatorValue, ModuleId, Persona,
};
use sobria_estimator::{
    CalibrationStatus, ForecastConfig, ForecastResult, ModelPreset, Openness, ParamOverrides,
    Scenario, ScenarioOutcome, SimulationRequest, SimulationResult, VendorDisclosure, VendorScope,
    VendorUnit, VisionPricing, YearlyForecastRequest, YearlyForecastResult, YearlyScenario,
    YearlyScenarioOutcome,
};
use sobria_geoloc::{
    CountryAggregate, DatacenterRecord, IndustrialSite, IndustrialSiteSummary, RegionFrAggregate,
    SankeyData, SankeyLink, SankeyNode,
};

// ─────────────────────────────────────────────────────────────────────────────
// meta_info
// ─────────────────────────────────────────────────────────────────────────────

/// App runtime information (footer + diagnostics).
#[derive(Debug, Clone, Serialize)]
pub struct MetaInfo {
    /// Semantic version of the `sobria-app` package.
    pub app_version: String,
    /// Configured Monte-Carlo seed (determinism).
    pub estimator_seed: u64,
    /// Number of Monte-Carlo draws (default 10⁴).
    pub estimator_n: u32,
    /// Path of the SQLite ledger (information only, no FS access).
    pub audit_path: String,
    /// User data root.
    pub data_root: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// referentiel (C26.5 — Gold access)
// ─────────────────────────────────────────────────────────────────────────────

/// Compact status of the Gold referentiel for the Settings page.
/// TS mirror in `web/src/lib/api.ts::ReferentielStatusDto`.
#[derive(Debug, Clone, Serialize)]
pub struct ReferentielStatusDto {
    /// `true` if the referentiel could be opened; otherwise the other fields
    /// may be empty.
    pub available: bool,
    /// Short human-readable message (why it is unavailable, or OK).
    pub message: String,
    /// Semantic version of the referentiel (inherited from `sobria-ingest`).
    pub version: String,
    /// Last modification date of the file (RFC 3339).
    pub snapshot_date: String,
    /// Hex SHA-256 of the SQLite file (integrity).
    pub sha256: String,
    /// Number of contributing sources.
    pub source_count: u64,
    /// Number of distinct models in `model_overview`.
    pub model_count: u64,
    /// Path of the SQLite file read (information only).
    pub path: String,
}

/// Result of an explicit reload request (`reload_referentiel`).
/// TS mirror in `web/src/lib/api.ts::ReferentielReloadResultDto`.
#[derive(Debug, Clone, Serialize)]
pub struct ReferentielReloadResultDto {
    /// `true` if `dvc pull` succeeded AND the referentiel is open.
    pub success: bool,
    /// Human-readable help or error message.
    pub message: String,
    /// Concatenated stdout/stderr of `dvc pull` (debug, max ~4 KB).
    pub dvc_output: String,
    /// Resulting status (None if the reload failed before opening).
    pub status: Option<ReferentielStatusDto>,
}

// ─────────────────────────────────────────────────────────────────────────────
// list_models
// ─────────────────────────────────────────────────────────────────────────────

/// Model preset sent to the frontend (static data turned into owned strings).
///
/// **C34.4** — 3 capability bools (vision/audio/reasoning) + `deprecated`:
/// `struct_excessive_bools` is allowed because each flag has a distinct,
/// independent meaning (see the matching doc on `ModelPreset`).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize)]
pub struct ModelPresetDto {
    pub id: String,
    pub display_name: String,
    pub provider: String,
    pub family: String,
    pub approx_params_billions: f64,
    pub openness: String,
    pub calibration: String,
    pub sources: Vec<String>,
    /// **C34.4** — Public release date (ISO `YYYY-MM-DD`).
    pub release_date: String,
    /// **C34.4** — Active parameters (in billions). `= approx_params_billions`
    /// for dense, lower for MoE.
    pub active_params_b: f64,
    /// **C34.4** — Typed vendor family (snake_case).
    pub model_family: String,
    /// **C34.4** — Architecture (`dense_transformer`, `moe`, `mamba`, `hybrid`).
    pub architecture: String,
    /// **C34.4** — Accepts image input.
    pub vision_capable: bool,
    /// **C34.4** — Accepts audio input.
    pub audio_capable: bool,
    /// **C34.4** — Reasoning model (built-in extended thinking / chain-of-thought).
    pub reasoning_capable: bool,
    /// **C34.4** — `(P5, P95)` of the thinking/output token ratio. `None` if
    /// not reasoning_capable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_token_multiplier: Option<(f64, f64)>,
    /// **C34.4** — Typical system overhead (vendor app interface).
    pub default_context_overhead_tokens: u32,
    /// **C34.4** — `true` for obsolete models (hidden by default in the UI).
    pub deprecated: bool,
    /// **C34.4** — Canonical URL of the vendor source (model card).
    pub source_url: String,
}

impl From<&ModelPreset> for ModelPresetDto {
    fn from(p: &ModelPreset) -> Self {
        use sobria_estimator::{ArchitectureKind, ModelFamily};
        Self {
            id: p.id.to_string(),
            display_name: p.display_name.to_string(),
            provider: p.provider.to_string(),
            family: p.family.to_string(),
            approx_params_billions: p.approx_params_billions,
            openness: match p.openness {
                Openness::Open => "open",
                Openness::OpenWeights => "open_weights",
                Openness::Closed => "closed",
            }
            .into(),
            calibration: match p.calibration {
                CalibrationStatus::Validated => "validated",
                CalibrationStatus::Indicative => "indicative",
                CalibrationStatus::Extrapolated => "extrapolated",
            }
            .into(),
            sources: p.sources.iter().map(|s| (*s).to_string()).collect(),
            release_date: p.release_date.to_string(),
            active_params_b: p.active_params_b,
            model_family: match p.model_family {
                ModelFamily::Anthropic => "anthropic",
                ModelFamily::OpenAi => "open_ai",
                ModelFamily::GoogleDeepMind => "google_deep_mind",
                ModelFamily::MetaAi => "meta_ai",
                ModelFamily::MistralAi => "mistral_ai",
                ModelFamily::DeepSeek => "deep_seek",
                ModelFamily::Xai => "xai",
                ModelFamily::Alibaba => "alibaba",
                ModelFamily::Microsoft => "microsoft",
                ModelFamily::Other => "other",
            }
            .into(),
            architecture: match p.architecture {
                ArchitectureKind::DenseTransformer => "dense_transformer",
                ArchitectureKind::Moe { .. } => "moe",
                ArchitectureKind::Mamba => "mamba",
                ArchitectureKind::Hybrid => "hybrid",
            }
            .into(),
            vision_capable: p.vision_capable,
            audio_capable: p.audio_capable,
            reasoning_capable: p.reasoning_capable,
            thinking_token_multiplier: p.thinking_token_multiplier,
            default_context_overhead_tokens: p.default_context_overhead_tokens,
            deprecated: p.deprecated,
            source_url: p.source_url.to_string(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// estimate_prompt
// ─────────────────────────────────────────────────────────────────────────────

/// Estimation request payload sent by the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EstimationRequestDto {
    pub model_id: String,
    pub tokens_in: u32,
    pub tokens_out_estimated: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datacenter_id: Option<String>,
    /// Methodology used for this computation (C24).
    ///
    /// `None` → falls back to the user's default methodology
    /// (`AppPreferencesDto::default_method`), or `EmpreinteMethod::default_method()`
    /// if no preference is stored yet.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<sobria_core::EmpreinteMethod>,
    /// **C34.3** — Input modalities of the prompt (text, vision, document,
    /// audio). `None`/absent → Text only (compatible with v0.8.x).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modalities: Option<Vec<sobria_core::InputModality>>,
    /// **C34.3** — System overhead (system prompt + tools + memory +
    /// thinking). `None`/absent → zeros (compatible with v0.8.x).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub overhead: Option<sobria_core::ContextOverhead>,
}

impl EstimationRequestDto {
    /// Converts into `sobria_core::EstimationRequest` (adds the timestamp).
    #[must_use]
    pub fn into_core(self, timestamp: DateTime<Utc>) -> EstimationRequest {
        EstimationRequest {
            model_id: self.model_id,
            tokens_in: self.tokens_in,
            tokens_out_estimated: self.tokens_out_estimated,
            datacenter_id: self.datacenter_id,
            timestamp,
            modalities: self.modalities.unwrap_or_default(),
            overhead: self.overhead.unwrap_or_default(),
        }
    }
}

/// Interval of one indicator (P5-P50-P95) + optional distribution
/// histogram.
#[derive(Debug, Clone, Serialize)]
pub struct IndicatorDto {
    /// Indicator name (`co2eq`, `energy`, `water`, `critical_metals`, `cost`).
    pub indicator: String,
    pub p5: f64,
    pub p50: f64,
    pub p95: f64,
    pub unit: String,
    /// Monte-Carlo histogram (equal-width). `None` for audit entries
    /// older than v0.2 or estimations with too small an N.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bins: Option<DistributionBins>,
}

impl From<&IndicatorValue> for IndicatorDto {
    fn from(v: &IndicatorValue) -> Self {
        Self {
            indicator: match v.indicator {
                Indicator::Co2Eq => "co2eq",
                Indicator::Energy => "energy",
                Indicator::Water => "water",
                Indicator::CriticalMetals => "critical_metals",
                Indicator::Cost => "cost",
            }
            .into(),
            p5: v.interval.p5,
            p50: v.interval.p50,
            p95: v.interval.p95,
            unit: v.unit.clone(),
            bins: v.bins.clone(),
        }
    }
}

/// Human-friendly equivalent (UI).
#[derive(Debug, Clone, Serialize)]
pub struct EquivalentDto {
    pub label: String,
    pub value: f64,
    pub source: String,
}

impl From<&Equivalent> for EquivalentDto {
    fn from(e: &Equivalent) -> Self {
        Self {
            label: e.label.clone(),
            value: e.value,
            source: e.source.clone(),
        }
    }
}

/// Clickable hypothesis (UI).
#[derive(Debug, Clone, Serialize)]
pub struct HypothesisDto {
    pub key: String,
    pub value: serde_json::Value,
    pub source: String,
}

impl From<&Hypothesis> for HypothesisDto {
    fn from(h: &Hypothesis) -> Self {
        Self {
            key: h.key.clone(),
            value: h.value.clone(),
            source: h.source.clone(),
        }
    }
}

/// Full estimation result returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct EstimationResultDto {
    /// Methodology used to produce this result (C24).
    /// Shown as a badge in `ResultBlock.svelte` and in the audit log.
    pub method: sobria_core::EmpreinteMethod,
    pub request: EstimationRequestEchoDto,
    pub indicators: Vec<IndicatorDto>,
    pub equivalents: Vec<EquivalentDto>,
    pub hypotheses: Vec<HypothesisDto>,
    pub computed_at: String,
    pub seed: u64,
    /// ID of the audit ledger entry that records this result.
    /// `0` = ephemeral estimation (not logged, see estimate_for_comparison).
    pub audit_id: i64,
}

/// Echo of the request (with the timestamp added server-side).
#[derive(Debug, Clone, Serialize)]
pub struct EstimationRequestEchoDto {
    pub model_id: String,
    pub tokens_in: u32,
    pub tokens_out_estimated: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datacenter_id: Option<String>,
    pub timestamp: String,
}

impl From<&EstimationRequest> for EstimationRequestEchoDto {
    fn from(r: &EstimationRequest) -> Self {
        Self {
            model_id: r.model_id.clone(),
            tokens_in: r.tokens_in,
            tokens_out_estimated: r.tokens_out_estimated,
            datacenter_id: r.datacenter_id.clone(),
            timestamp: r.timestamp.to_rfc3339(),
        }
    }
}

impl EstimationResultDto {
    /// Builds the DTO from an `EstimationResult` and the ledger `audit_id`.
    #[must_use]
    pub fn from_result(r: &EstimationResult, audit_id: i64) -> Self {
        Self {
            method: r.method,
            request: (&r.request).into(),
            indicators: r.indicators.iter().map(Into::into).collect(),
            equivalents: r.equivalents.iter().map(Into::into).collect(),
            hypotheses: r.hypotheses.iter().map(Into::into).collect(),
            computed_at: r.computed_at.to_rfc3339(),
            seed: r.seed,
            audit_id,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// audit
// ─────────────────────────────────────────────────────────────────────────────

/// Integrity report of the audit chain.
#[derive(Debug, Clone, Serialize)]
pub struct IntegrityReportDto {
    pub total_entries: usize,
    pub valid: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_invalid_id: Option<i64>,
    pub message: String,
}

impl From<&IntegrityReport> for IntegrityReportDto {
    fn from(r: &IntegrityReport) -> Self {
        Self {
            total_entries: r.total_entries,
            valid: r.valid,
            first_invalid_id: r.first_invalid_id,
            message: r.message.clone(),
        }
    }
}

/// Summary of one ledger entry (for the paginated list).
#[derive(Debug, Clone, Serialize)]
pub struct AuditEntrySummaryDto {
    pub id: i64,
    pub timestamp: String,
    pub model_id: String,
    pub co2eq_p50: f64,
    pub sig_short: String,
    pub purged: bool,
    /// Methodology that produced this entry (C24).
    /// Extracted from the JSON payload. For historical pre-C24 entries it is
    /// `AfnorSobria` (the only engine available then) through the
    /// `#[serde(default)]` on `EstimationResult.method`.
    pub method: sobria_core::EmpreinteMethod,
}

impl AuditEntrySummaryDto {
    /// Builds the summary by extracting `model_id` + `co2eq_p50` + `method`
    /// from the payload. If the payload is purged or malformed, it is filled
    /// with sentinels (method → AfnorSobria by default, consistent with
    /// historical ledgers).
    #[must_use]
    pub fn from_entry(e: &AuditEntry) -> Self {
        let parsed = parse_payload_full(&e.estimation_result_json);
        let sig_short = e.sig.chars().take(16).collect();
        Self {
            id: e.id,
            timestamp: e.timestamp.to_rfc3339(),
            model_id: parsed.model_id,
            co2eq_p50: parsed.co2eq_p50,
            sig_short,
            purged: e.is_purged(),
            method: parsed.method,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// territoire_fr (C13 — M20 French territory)
// ─────────────────────────────────────────────────────────────────────────────

/// Summary of an industrial site for the M20 map.
#[derive(Debug, Clone, Serialize)]
pub struct IndustrialSiteSummaryDto {
    pub code_iris: String,
    pub commune: String,
    pub department_code: String,
    pub region_iso: String,
    pub lat: f64,
    pub lon: f64,
    pub consumption_mwh_elec: f64,
    pub consumption_mwh_gas: f64,
    pub consumption_total_mwh: f64,
    pub pdl_total: u32,
    pub year: u32,
}

impl From<&IndustrialSite> for IndustrialSiteSummaryDto {
    fn from(s: &IndustrialSite) -> Self {
        Self {
            code_iris: s.code_iris.clone(),
            commune: s.commune.clone(),
            department_code: s.department_code.clone(),
            region_iso: s.region_iso.clone(),
            lat: s.lat,
            lon: s.lon,
            consumption_mwh_elec: s.consumption_mwh_elec,
            consumption_mwh_gas: s.consumption_mwh_gas_grtgaz + s.consumption_mwh_gas_terega,
            consumption_total_mwh: s.consumption_total_mwh,
            pdl_total: s.pdl_total,
            year: s.year,
        }
    }
}

/// French regional aggregate for the drill-down.
#[derive(Debug, Clone, Serialize)]
pub struct RegionFrAggregateDto {
    pub region_iso: String,
    pub region_name: String,
    pub insee_code: String,
    pub site_count: usize,
    pub total_consumption_mwh_elec: f64,
    pub total_consumption_mwh_gas: f64,
    pub total_consumption_mwh: f64,
    pub centroid_lat: f64,
    pub centroid_lon: f64,
    pub nuclear_share_pct: f64,
    pub top_sites: Vec<TopSiteDto>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopSiteDto {
    pub code_iris: String,
    pub commune: String,
    pub consumption_total_mwh: f64,
}

impl From<&IndustrialSiteSummary> for TopSiteDto {
    fn from(s: &IndustrialSiteSummary) -> Self {
        Self {
            code_iris: s.code_iris.clone(),
            commune: s.commune.clone(),
            consumption_total_mwh: s.consumption_total_mwh,
        }
    }
}

impl From<&RegionFrAggregate> for RegionFrAggregateDto {
    fn from(r: &RegionFrAggregate) -> Self {
        Self {
            region_iso: r.region_iso.clone(),
            region_name: r.region_name.clone(),
            insee_code: r.insee_code.clone(),
            site_count: r.site_count,
            total_consumption_mwh_elec: r.total_consumption_mwh_elec,
            total_consumption_mwh_gas: r.total_consumption_mwh_gas,
            total_consumption_mwh: r.total_consumption_mwh,
            centroid_lat: r.centroid_lat,
            centroid_lon: r.centroid_lon,
            nuclear_share_pct: r.nuclear_share_pct,
            top_sites: r.top_sites.iter().map(Into::into).collect(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// sankey_fr (C13)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct SankeyNodeDto {
    pub id: String,
    pub label: String,
    pub layer: u8,
    pub value_twh: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SankeyLinkDto {
    pub source: String,
    pub target: String,
    pub value_twh: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct SankeyDataDto {
    pub nodes: Vec<SankeyNodeDto>,
    pub links: Vec<SankeyLinkDto>,
    pub total_production_twh: f64,
    pub year: u32,
    pub source_url: String,
    pub source_sha256: String,
}

impl From<&SankeyNode> for SankeyNodeDto {
    fn from(n: &SankeyNode) -> Self {
        Self {
            id: n.id.clone(),
            label: n.label.clone(),
            layer: n.layer,
            value_twh: n.value_twh,
        }
    }
}

impl From<&SankeyLink> for SankeyLinkDto {
    fn from(l: &SankeyLink) -> Self {
        Self {
            source: l.source.clone(),
            target: l.target.clone(),
            value_twh: l.value_twh,
        }
    }
}

impl From<&SankeyData> for SankeyDataDto {
    fn from(s: &SankeyData) -> Self {
        Self {
            nodes: s.nodes.iter().map(Into::into).collect(),
            links: s.links.iter().map(Into::into).collect(),
            total_production_twh: s.total_production_twh,
            year: s.year,
            source_url: s.source_url.clone(),
            source_sha256: s.source_sha256.clone(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// batch CSV (C21 — M18)
// ─────────────────────────────────────────────────────────────────────────────

/// Batch CSV processing request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BatchRequestDto {
    /// Absolute path to the input CSV.
    pub input_csv_path: String,
    /// If set: writes a results CSV at this path.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_csv_path: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchAggregateDto {
    pub total_co2eq_g_p50: f64,
    pub total_energy_wh_p50: f64,
    pub total_water_l_p50: f64,
    pub avg_co2eq_g_p50: f64,
    pub min_co2eq_g_p50: f64,
    pub max_co2eq_g_p50: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchModelAggregateDto {
    pub model_id: String,
    pub count: u32,
    pub total_co2eq_g_p50: f64,
    pub avg_co2eq_g_p50: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct BatchResultDto {
    pub rows_processed: u32,
    pub rows_rejected: u32,
    pub aggregate: BatchAggregateDto,
    pub by_model: Vec<BatchModelAggregateDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output_csv_path: Option<String>,
    pub first_audit_id: i64,
    pub last_audit_id: i64,
}

// ─────────────────────────────────────────────────────────────────────────────
// projects + datasheet (C20 — M17 project footprint)
// ─────────────────────────────────────────────────────────────────────────────

/// Project representation sent to the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectDto {
    pub id: i64,
    pub name: String,
    pub description: String,
    /// RFC 3339 UTC.
    pub period_start: String,
    pub period_end: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub updated_at: String,
}

/// Creation payload.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateProjectDto {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub period_start: String,
    pub period_end: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Partial update payload (dates cannot be changed — see brief §1.1).
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UpdateProjectDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<String>>,
}

/// Aggregated composition returned with the datasheet.
#[derive(Debug, Clone, Serialize)]
pub struct CompositionDto {
    pub total_requests: u32,
    pub unique_models: Vec<String>,
    pub total_co2eq_g_p50: f64,
    pub total_energy_wh_p50: f64,
    pub total_water_l_p50: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_first_entry: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_last_entry: Option<String>,
    /// Methodologies used by the entries of the
    /// period. Surfaced by the M17 UI as information for researchers.
    pub methodologies_used: Vec<sobria_core::EmpreinteMethod>,
}

impl From<&sobria_export::Composition> for CompositionDto {
    fn from(c: &sobria_export::Composition) -> Self {
        Self {
            total_requests: c.total_requests,
            unique_models: c.unique_models.clone(),
            total_co2eq_g_p50: c.total_co2eq_g_p50,
            total_energy_wh_p50: c.total_energy_wh_p50,
            total_water_l_p50: c.total_water_l_p50,
            date_first_entry: c.date_first_entry.map(|d| d.to_rfc3339()),
            date_last_entry: c.date_last_entry.map(|d| d.to_rfc3339()),
            methodologies_used: c.methodologies_used.clone(),
        }
    }
}

/// Gebru datasheet produced for a project.
#[derive(Debug, Clone, Serialize)]
pub struct DatasheetDto {
    pub project: ProjectDto,
    pub jsonld: serde_json::Value,
    pub composition: CompositionDto,
    pub sha256: String,
}

// ─────────────────────────────────────────────────────────────────────────────
// dashboard + eco-budget (C19 — M15 + M25)
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize)]
pub struct DashboardComparisonDto {
    pub previous_total_co2eq_g_p50: f64,
    pub delta_co2eq_pct: f64,
    pub previous_total_requests: u32,
    pub delta_requests_pct: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TopModelDto {
    pub model_id: String,
    pub request_count: u32,
    pub total_co2eq_g_p50: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DailySeriesPointDto {
    pub date: String,
    pub request_count: u32,
    pub co2eq_g_p50: f64,
    pub energy_wh_p50: f64,
    pub water_l_p50: f64,
}

/// Aggregated total for a single methodology over the period.
#[derive(Debug, Clone, Serialize)]
pub struct MethodTotalDto {
    pub method: sobria_core::EmpreinteMethod,
    pub request_count: u32,
    pub total_co2eq_g_p50: f64,
    pub total_energy_wh_p50: f64,
    pub total_water_l_p50: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct DashboardSummaryDto {
    pub period_label: String,
    pub period_start: String,
    pub period_end: String,
    pub total_requests: u32,
    pub total_co2eq_g_p50: f64,
    pub total_energy_wh_p50: f64,
    pub total_water_l_p50: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vs_previous: Option<DashboardComparisonDto>,
    pub top_models: Vec<TopModelDto>,
    pub daily_series: Vec<DailySeriesPointDto>,
    /// Breakdown by methodology present in the period.
    pub method_breakdown: Vec<MethodTotalDto>,
    /// `true` if the period contains more than one methodology. The frontend
    /// then shows a warning: summing 2 methodologies is not scientific.
    pub warning_multi_method: bool,
}

impl From<&crate::dashboard::DashboardSummary> for DashboardSummaryDto {
    fn from(s: &crate::dashboard::DashboardSummary) -> Self {
        Self {
            period_label: s.period_label.clone(),
            period_start: s.period_start.to_rfc3339(),
            period_end: s.period_end.to_rfc3339(),
            total_requests: s.total_requests,
            total_co2eq_g_p50: s.total_co2eq_g_p50,
            total_energy_wh_p50: s.total_energy_wh_p50,
            total_water_l_p50: s.total_water_l_p50,
            vs_previous: s.vs_previous.as_ref().map(|v| DashboardComparisonDto {
                previous_total_co2eq_g_p50: v.previous_total_co2eq_g_p50,
                delta_co2eq_pct: v.delta_co2eq_pct,
                previous_total_requests: v.previous_total_requests,
                delta_requests_pct: v.delta_requests_pct,
            }),
            top_models: s
                .top_models
                .iter()
                .map(|m| TopModelDto {
                    model_id: m.model_id.clone(),
                    request_count: m.request_count,
                    total_co2eq_g_p50: m.total_co2eq_g_p50,
                })
                .collect(),
            daily_series: s
                .daily_series
                .iter()
                .map(|p| DailySeriesPointDto {
                    date: p.date.clone(),
                    request_count: p.request_count,
                    co2eq_g_p50: p.co2eq_g_p50,
                    energy_wh_p50: p.energy_wh_p50,
                    water_l_p50: p.water_l_p50,
                })
                .collect(),
            method_breakdown: s
                .method_breakdown
                .iter()
                .map(|m| MethodTotalDto {
                    method: m.method,
                    request_count: m.request_count,
                    total_co2eq_g_p50: m.total_co2eq_g_p50,
                    total_energy_wh_p50: m.total_energy_wh_p50,
                    total_water_l_p50: m.total_water_l_p50,
                })
                .collect(),
            warning_multi_method: s.warning_multi_method,
        }
    }
}

/// Personal goal exchanged with the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersonalGoalDto {
    /// "co2eq" | "energy" | "water"
    pub indicator: String,
    /// "daily" | "weekly" | "monthly"
    pub period: String,
    pub value_max: f64,
    pub unit: String,
}

/// Consumption status of a goal.
#[derive(Debug, Clone, Serialize)]
pub struct BudgetStatusDto {
    pub goal: PersonalGoalDto,
    pub current_value: f64,
    pub period_start: String,
    pub period_end: String,
    /// 0..100+ (may go above 100).
    pub consumed_pct: f64,
    /// "ok" (<70%), "warning" (70-100%), "exceeded" (>100%)
    pub status: String,
    /// value_max - current_value (may be < 0).
    pub remaining: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// model detail (C18 — M9 model referentiel)
// ─────────────────────────────────────────────────────────────────────────────

/// P5/P50/P95 triplet (equivalent to `UncertaintyInterval` without
/// validation on the DTO side).
#[derive(Debug, Clone, Serialize)]
pub struct TripletDto {
    pub p5: f64,
    pub p50: f64,
    pub p95: f64,
}

/// Detail sheet of a model, exposing its distribution params and
/// a contextual baseline (gpt-4o-mini 100/500 tokens, default
/// parameters). **Not logged** in the audit ledger.
///
/// **C34.5** — extended with the model capabilities (vision, audio,
/// reasoning, MoE) to feed the M9 sheet.
///
/// 4 independent bools: `vision_capable`, `audio_capable`,
/// `reasoning_capable`, `deprecated` (see the matching doc on `ModelPreset`).
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone, Serialize)]
pub struct ModelDetailDto {
    pub id: String,
    pub display_name: String,
    pub provider: String,
    pub family: String,
    pub approx_params_billions: f64,
    pub openness: String,
    pub calibration: String,
    pub sources: Vec<String>,
    /// P5/P50/P95 range of the prefill energy (mJ per input token).
    pub epsilon_prefill_mj_per_token: TripletDto,
    /// P5/P50/P95 range of the decode energy (mJ per output token).
    pub epsilon_decode_mj_per_token: TripletDto,
    /// P5/P50/P95 range of the amortised embodied carbon (gCO₂eq per request).
    pub embodied_g_per_request: TripletDto,
    /// CO₂eq P5/P50/P95 for the reference prompt (100 in / 500 out).
    pub baseline_co2eq_p5_g: f64,
    pub baseline_co2eq_p50_g: f64,
    pub baseline_co2eq_p95_g: f64,
    pub baseline_energy_wh_p50: f64,
    pub baseline_water_l_p50: f64,
    /// **C32.4** — Official disclosures published by the vendor.
    /// Empty for models whose vendor has not published any (Anthropic,
    /// OpenAI as of 2026-05).
    pub vendor_disclosures: Vec<VendorDisclosureDto>,
    /// **C34.5** — Public release date (ISO `YYYY-MM-DD`).
    pub release_date: String,
    /// **C34.5** — Active parameters (= total for dense, lower for MoE).
    pub active_params_b: f64,
    /// **C34.5** — Typed family (snake_case).
    pub model_family: String,
    /// **C34.5** — Architecture (`dense_transformer` / `moe` / `mamba` /
    /// `hybrid`).
    pub architecture: String,
    /// **C34.5** — Number of experts if MoE (otherwise `None`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moe_experts: Option<u32>,
    /// **C34.5** — Number of active experts per token if MoE (otherwise `None`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub moe_active_experts: Option<u32>,
    /// **C34.5** — Accepts image input.
    pub vision_capable: bool,
    /// **C34.5** — Vision token pricing (vendor formula). `None` if
    /// `vision_capable = false`. Serialized as tagged JSON (`{"kind": …}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vision_pricing: Option<VisionPricing>,
    /// **C34.5** — Accepts audio input.
    pub audio_capable: bool,
    /// **C34.5** — Built-in reasoning model.
    pub reasoning_capable: bool,
    /// **C34.5** — `(P5, P95)` thinking/output token ratio. `None` if
    /// not reasoning.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub thinking_token_multiplier: Option<(f64, f64)>,
    /// **C34.5** — Typical system overhead of the vendor app interface.
    pub default_context_overhead_tokens: u32,
    /// **C34.5** — Obsolete model (hidden by default in the UI).
    pub deprecated: bool,
    /// **C34.5** — Canonical URL of the vendor source.
    pub source_url: String,
}

/// **C32.4** — Official figure published by a vendor (Mistral × ADEME,
/// Google Gemini, Meta Llama). See `sobria_estimator::VendorDisclosure`.
#[derive(Debug, Clone, Serialize)]
pub struct VendorDisclosureDto {
    pub vendor: String,
    /// `"training"` | `"inference_per_prompt"`.
    pub scope: String,
    pub value: f64,
    /// `"t_co2eq"` | `"g_co2eq"` | `"wh"` | `"ml_water"` | `"m3_water"`.
    pub unit: String,
    pub source_url: String,
    pub published_at: String,
    pub methodology_note: String,
}

/// **C32.4** — Row of the vendor disclosure comparison table (M9 main
/// page). Aggregates the disclosures of all models of a given
/// vendor.
#[derive(Debug, Clone, Serialize)]
pub struct VendorComparisonRowDto {
    /// Vendor name (e.g. `"Mistral AI"`, `"OpenAI"`).
    pub vendor: String,
    /// `true` if at least one model of the vendor publishes a
    /// prompt-level disclosure (inference_per_prompt).
    pub has_prompt_level: bool,
    /// `true` if at least one model of the vendor publishes a training
    /// disclosure.
    pub has_training: bool,
    /// First source URL found (if at least one disclosure exists).
    /// `None` if no model of the vendor has a disclosure.
    pub primary_source_url: Option<String>,
}

impl From<&VendorDisclosure> for VendorDisclosureDto {
    fn from(d: &VendorDisclosure) -> Self {
        Self {
            vendor: d.vendor.to_string(),
            scope: match d.scope {
                VendorScope::Training => "training",
                VendorScope::InferencePerPrompt => "inference_per_prompt",
            }
            .into(),
            value: d.value,
            unit: match d.unit {
                VendorUnit::TCo2Eq => "t_co2eq",
                VendorUnit::GCo2Eq => "g_co2eq",
                VendorUnit::Wh => "wh",
                VendorUnit::MlWater => "ml_water",
                VendorUnit::M3Water => "m3_water",
            }
            .into(),
            source_url: d.source_url.to_string(),
            published_at: d.published_at.to_string(),
            methodology_note: d.methodology_note.to_string(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// benchmark (C17 — M3 compare models)
// ─────────────────────────────────────────────────────────────────────────────

/// Benchmark request for N models on the same prompt.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRequestDto {
    /// 1..=20 model_ids to compare.
    pub model_ids: Vec<String>,
    pub tokens_in: u32,
    pub tokens_out_estimated: u32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub datacenter_id: Option<String>,
}

/// Outcome of one benchmarked model.
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkOutcomeDto {
    pub model_id: String,
    pub display_name: String,
    pub provider: String,
    pub family: String,
    pub openness: String,
    pub calibration: String,
    pub result: EstimationResultDto,
    pub rank_co2eq: u32,
    pub rank_energy: u32,
    pub rank_water: u32,
}

/// Overall result of a benchmark.
#[derive(Debug, Clone, Serialize)]
pub struct BenchmarkResultDto {
    pub outcomes: Vec<BenchmarkOutcomeDto>,
    pub ranking_by_co2eq_p50: Vec<String>,
    pub ranking_by_energy_p50: Vec<String>,
    pub ranking_by_water_p50: Vec<String>,
    pub tokens_in: u32,
    pub tokens_out_estimated: u32,
}

// ─────────────────────────────────────────────────────────────────────────────
// yearly forecast (C15 — M16 12-month forecaster)
// ─────────────────────────────────────────────────────────────────────────────

/// Forecast payload sent by the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearlyForecastRequestDto {
    pub baseline: EstimationRequestDto,
    pub scenarios: Vec<YearlyScenarioDto>,
    pub months: u32,
    pub base_volume_per_day: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct YearlyScenarioDto {
    pub label: String,
    pub monthly_growth_pct: f64,
}

impl From<YearlyScenarioDto> for YearlyScenario {
    fn from(d: YearlyScenarioDto) -> Self {
        Self {
            label: d.label,
            monthly_growth_pct: d.monthly_growth_pct,
        }
    }
}

impl YearlyForecastRequestDto {
    /// Converts into internal types (adds the baseline timestamp).
    #[must_use]
    pub fn into_core(self, baseline_timestamp: DateTime<Utc>) -> YearlyForecastRequest {
        YearlyForecastRequest {
            baseline: self.baseline.into_core(baseline_timestamp),
            scenarios: self.scenarios.into_iter().map(Into::into).collect(),
            months: self.months,
            base_volume_per_day: self.base_volume_per_day,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct YearlyScenarioOutcomeDto {
    pub label: String,
    pub monthly_growth_pct: f64,
    pub monthly_p5_g: Vec<f64>,
    pub monthly_p50_g: Vec<f64>,
    pub monthly_p95_g: Vec<f64>,
    pub cumulative_p5_g: Vec<f64>,
    pub cumulative_p50_g: Vec<f64>,
    pub cumulative_p95_g: Vec<f64>,
    pub annual_p5_g: f64,
    pub annual_p50_g: f64,
    pub annual_p95_g: f64,
}

impl From<&YearlyScenarioOutcome> for YearlyScenarioOutcomeDto {
    fn from(o: &YearlyScenarioOutcome) -> Self {
        Self {
            label: o.label.clone(),
            monthly_growth_pct: o.monthly_growth_pct,
            monthly_p5_g: o.monthly_p5_g.clone(),
            monthly_p50_g: o.monthly_p50_g.clone(),
            monthly_p95_g: o.monthly_p95_g.clone(),
            cumulative_p5_g: o.cumulative_p5_g.clone(),
            cumulative_p50_g: o.cumulative_p50_g.clone(),
            cumulative_p95_g: o.cumulative_p95_g.clone(),
            annual_p5_g: o.annual_p5_g,
            annual_p50_g: o.annual_p50_g,
            annual_p95_g: o.annual_p95_g,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct YearlyForecastResultDto {
    pub baseline_co2eq_p5_g: f64,
    pub baseline_co2eq_p50_g: f64,
    pub baseline_co2eq_p95_g: f64,
    /// `audit_id` of the entry logged for the baseline (0 if not logged).
    pub baseline_audit_id: i64,
    pub scenarios: Vec<YearlyScenarioOutcomeDto>,
}

impl YearlyForecastResultDto {
    #[must_use]
    pub fn from_result(r: &YearlyForecastResult, baseline_audit_id: i64) -> Self {
        Self {
            baseline_co2eq_p5_g: r.baseline_co2eq_p5_g,
            baseline_co2eq_p50_g: r.baseline_co2eq_p50_g,
            baseline_co2eq_p95_g: r.baseline_co2eq_p95_g,
            baseline_audit_id,
            scenarios: r.scenarios.iter().map(Into::into).collect(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// CSRD report (C14 — M22)
// ─────────────────────────────────────────────────────────────────────────────

/// CSRD/AGEC report generation request sent by the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CsrdReportRequestDto {
    /// ISO 8601 (`2026-01-01T00:00:00Z`).
    pub period_start: String,
    pub period_end: String,
    pub organization_name: String,
    /// UI locale — v1.0: `"fr"`.
    pub locale: String,
}

/// Response returned after generation.
#[derive(Debug, Clone, Serialize)]
pub struct CsrdReportResultDto {
    pub pdf_path: String,
    pub provo_path: String,
    pub pdf_sha256: String,
    pub audit_entries_count: usize,
    pub total_requests: u32,
    pub total_co2eq_g_p50: f64,
    pub total_energy_wh_p50: f64,
    pub total_water_l_p50: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// simulation (C11 — M13 "what if" simulator)
// ─────────────────────────────────────────────────────────────────────────────

/// Simulation payload sent by the frontend.
///
/// See `docs/archive/chantiers/C11-simulateur-et-si.md`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationRequestDto {
    pub baseline: EstimationRequestDto,
    pub scenarios: Vec<ScenarioDto>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub forecast: Option<ForecastConfigDto>,
}

/// Scenario sent by the frontend.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioDto {
    pub label: String,
    #[serde(default)]
    pub overrides: ParamOverridesDto,
}

/// Optional parameter overrides. All fields are optional.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParamOverridesDto {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tokens_out: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pue: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub if_electrical_g_per_kwh: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub embodied_g_per_request: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wue_l_per_kwh: Option<f64>,
}

impl From<ParamOverridesDto> for ParamOverrides {
    fn from(d: ParamOverridesDto) -> Self {
        Self {
            model_id: d.model_id,
            tokens_out: d.tokens_out,
            pue: d.pue,
            if_electrical_g_per_kwh: d.if_electrical_g_per_kwh,
            embodied_g_per_request: d.embodied_g_per_request,
            wue_l_per_kwh: d.wue_l_per_kwh,
        }
    }
}

impl From<ScenarioDto> for Scenario {
    fn from(d: ScenarioDto) -> Self {
        Self {
            label: d.label,
            overrides: d.overrides.into(),
        }
    }
}

/// 12-month projection config.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForecastConfigDto {
    pub months: u32,
    pub monthly_growth_pct: f64,
    pub base_volume_per_day: f64,
}

impl From<ForecastConfigDto> for ForecastConfig {
    fn from(d: ForecastConfigDto) -> Self {
        Self {
            months: d.months,
            monthly_growth_pct: d.monthly_growth_pct,
            base_volume_per_day: d.base_volume_per_day,
        }
    }
}

impl SimulationRequestDto {
    /// Converts into the internal `SimulationRequest`, adding the baseline timestamp.
    #[must_use]
    pub fn into_core(self, baseline_timestamp: DateTime<Utc>) -> SimulationRequest {
        SimulationRequest {
            baseline: self.baseline.into_core(baseline_timestamp),
            scenarios: self.scenarios.into_iter().map(Into::into).collect(),
            forecast: self.forecast.map(Into::into),
        }
    }
}

// ── Output ───────────────────────────────────────────────────────────────────

/// Result of one scenario, ready to display.
#[derive(Debug, Clone, Serialize)]
pub struct ScenarioOutcomeDto {
    pub label: String,
    pub result: EstimationResultDto,
    /// Δ versus the baseline P50, in gCO₂eq (may be negative).
    pub delta_co2eq_g: f64,
    /// Relative Δ as a percentage of the baseline P50.
    pub delta_pct: f64,
}

/// Result of a 12-month projection.
#[derive(Debug, Clone, Serialize)]
pub struct ForecastResultDto {
    pub months: u32,
    pub base_volume_per_day: f64,
    pub monthly_growth_pct: f64,
    pub baseline_monthly_co2eq_g: Vec<f64>,
    pub baseline_annual_co2eq_g: f64,
    pub scenarios_annual_co2eq_g: Vec<f64>,
}

impl From<ForecastResult> for ForecastResultDto {
    fn from(r: ForecastResult) -> Self {
        Self {
            months: r.months,
            base_volume_per_day: r.base_volume_per_day,
            monthly_growth_pct: r.monthly_growth_pct,
            baseline_monthly_co2eq_g: r.baseline_monthly_co2eq_g,
            baseline_annual_co2eq_g: r.baseline_annual_co2eq_g,
            scenarios_annual_co2eq_g: r.scenarios_annual_co2eq_g,
        }
    }
}

/// Overall result of a simulation.
#[derive(Debug, Clone, Serialize)]
pub struct SimulationResultDto {
    /// Baseline estimation (with a real `audit_id` — logged).
    pub baseline: EstimationResultDto,
    /// Outcomes per scenario. `result.audit_id == 0` (not logged).
    pub scenarios: Vec<ScenarioOutcomeDto>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub forecast: Option<ForecastResultDto>,
}

impl SimulationResultDto {
    /// Builds the output DTO, passing the `audit_id` of the baseline entry
    /// in the ledger. Scenarios get `audit_id = 0` (not logged
    /// — see brief C11 §3.audit).
    #[must_use]
    pub fn from_result(r: &SimulationResult, baseline_audit_id: i64) -> Self {
        let baseline = EstimationResultDto::from_result(&r.baseline, baseline_audit_id);
        let scenarios = r
            .scenarios
            .iter()
            .map(|o: &ScenarioOutcome| ScenarioOutcomeDto {
                label: o.label.clone(),
                result: EstimationResultDto::from_result(&o.result, 0),
                delta_co2eq_g: o.delta_co2eq_g,
                delta_pct: o.delta_pct,
            })
            .collect();
        Self {
            baseline,
            scenarios,
            forecast: r.forecast.clone().map(Into::into),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// datacenters (C12 — M12)
// ─────────────────────────────────────────────────────────────────────────────

/// Summary of a datacenter, enough to place a marker on the map.
#[derive(Debug, Clone, Serialize)]
pub struct DatacenterSummaryDto {
    pub id: String,
    pub name: String,
    pub operator: String,
    pub country_iso: String,
    pub city: String,
    pub lat: f64,
    pub lon: f64,
    pub pue: f64,
    pub if_electrical_g_per_kwh: f64,
}

impl From<&DatacenterRecord> for DatacenterSummaryDto {
    fn from(d: &DatacenterRecord) -> Self {
        Self {
            id: d.id.clone(),
            name: d.name.clone(),
            operator: d.operator.clone(),
            country_iso: d.country_iso.clone(),
            city: d.city.clone(),
            lat: d.lat,
            lon: d.lon,
            pue: d.pue,
            if_electrical_g_per_kwh: d.if_electrical_g_per_kwh,
        }
    }
}

/// Full detail for the drill-down (donut + bars + 24h).
#[derive(Debug, Clone, Serialize)]
pub struct DatacenterDetailDto {
    pub id: String,
    pub name: String,
    pub operator: String,
    pub country_iso: String,
    pub city: String,
    pub lat: f64,
    pub lon: f64,
    pub pue: f64,
    pub if_electrical_g_per_kwh: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wue_l_per_kwh: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capacity_mw: Option<f64>,
    pub sources: Vec<String>,
    pub hourly_profile_24h: Vec<f64>,
    /// CO₂eq P50 (gCO₂eq) for a reference prompt (gpt-4o-mini 100/500 tokens)
    /// with the DC's PUE/IF/WUE parameters. Lets the UI fill the
    /// "bars" without a new IPC command.
    pub baseline_co2eq_p50_g: f64,
    /// Same for energy (Wh) — median.
    pub baseline_energy_wh_p50: f64,
    /// Same for water (L) — median.
    pub baseline_water_l_p50: f64,
}

/// Per-country aggregate for the zoomed-out Europe view.
#[derive(Debug, Clone, Serialize)]
pub struct CountryAggregateDto {
    pub country_iso: String,
    pub datacenter_count: usize,
    pub avg_pue: f64,
    pub if_electrical_g_per_kwh: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_capacity_mw: Option<f64>,
    pub centroid_lat: f64,
    pub centroid_lon: f64,
}

impl From<&CountryAggregate> for CountryAggregateDto {
    fn from(c: &CountryAggregate) -> Self {
        Self {
            country_iso: c.country_iso.clone(),
            datacenter_count: c.datacenter_count,
            avg_pue: c.avg_pue,
            if_electrical_g_per_kwh: c.if_electrical_g_per_kwh,
            total_capacity_mw: c.total_capacity_mw,
            centroid_lat: c.centroid_lat,
            centroid_lon: c.centroid_lon,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// preferences (C10 — ADR-0010)
// ─────────────────────────────────────────────────────────────────────────────

/// User preferences shared between Rust and the SvelteKit frontend.
///
/// See [ADR-0010](../../docs/adr/ADR-0010-personas-and-module-gating.md) and
/// `docs/archive/chantiers/C10-onboarding-personas.md` §2.2.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppPreferencesDto {
    /// Current persona (`null` until the user has completed onboarding).
    #[serde(default)]
    pub persona: Option<Persona>,
    /// Modules visible in the UI rail. Closed set v1.3 (24 possible IDs, M4 reserved).
    pub enabled_modules: Vec<ModuleId>,
    /// `true` once onboarding has been completed at least once.
    pub onboarded: bool,
    /// UI language: `"fr"` or `"en"`.
    pub lang: String,
    /// Default methodology for computations (C24).
    /// AFNOR SPEC 2314 (Sobr.ia) on first launch — sovereign choice.
    #[serde(default)]
    pub default_method: sobria_core::EmpreinteMethod,
    /// Extra methodologies shown for reference in the
    /// "Voir aussi" panel (C24). Empty list by default.
    #[serde(default)]
    pub also_show_methods: Vec<sobria_core::EmpreinteMethod>,
    /// Last selected datacenter, prefilled on the next load of the
    /// /estimate, /comparer, /simuler routes (C25). `None` = no
    /// prefill, the user starts from an empty picker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default_datacenter_id: Option<String>,
}

impl AppPreferencesDto {
    /// Defaults returned when `app_preferences` is empty
    /// (first launch). Uses the `pro_tech` bundle, the most
    /// balanced one (see ADR-0010 §"Onboarding non-bloquant").
    #[must_use]
    pub fn defaults() -> Self {
        Self {
            persona: None,
            enabled_modules: Persona::ProTech.default_modules(),
            onboarded: false,
            lang: "fr".into(),
            default_method: sobria_core::EmpreinteMethod::default_method(),
            also_show_methods: Vec::new(),
            default_datacenter_id: None,
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Methodology catalogue (C24)
// ─────────────────────────────────────────────────────────────────────────────

/// Metadata of an LLM footprint methodology, exposed to the frontend
/// for display in `Settings → Méthodologies` (page `/methodologies`).
///
/// See [`sobria_estimator::MethodologyInfo`] on the backend for the source
/// of truth; this DTO is a 1:1 serializable projection of it.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodologyInfoDto {
    /// Stable identifier (`afnor_sobria` / `ecologits`).
    pub method: sobria_core::EmpreinteMethod,
    /// Name shown in the UI (French).
    pub display_name: String,
    /// Short 1-2 sentence description (French).
    pub short_description: String,
    /// Reference URL (DOI or official doc).
    pub reference_url: String,
    /// Normalized DOI if available.
    pub doi: Option<String>,
    /// License of the published methodology.
    pub license: String,
    /// Calibration status ("peer_reviewed_reproduced" /
    /// "public_method_calibration_pending" / "indicative").
    pub calibration: String,
    /// Publication year of the reference methodology.
    pub year_published: u16,
    /// Organization that maintains the reference methodology.
    pub maintained_by: String,
}

impl From<&sobria_estimator::MethodologyInfo> for MethodologyInfoDto {
    fn from(info: &sobria_estimator::MethodologyInfo) -> Self {
        let calibration = match info.calibration {
            sobria_estimator::MethodologyCalibration::PeerReviewedReproduced => {
                "peer_reviewed_reproduced"
            },
            sobria_estimator::MethodologyCalibration::PublicMethodCalibrationPending => {
                "public_method_calibration_pending"
            },
            sobria_estimator::MethodologyCalibration::Indicative => "indicative",
        };
        Self {
            method: info.method,
            display_name: info.display_name.into(),
            short_description: info.short_description.into(),
            reference_url: info.reference_url.into(),
            doi: info.doi.map(Into::into),
            license: info.license.into(),
            calibration: calibration.into(),
            year_published: info.year_published,
            maintained_by: info.maintained_by.into(),
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// browser extension — pairing (C27.5.c/d)
// ─────────────────────────────────────────────────────────────────────────────

/// 6-digit code shown to the user in the Sobr.ia UI to pair
/// the browser extension. TTL 5 min.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingCodeDto {
    /// The 6 digits to type into the extension.
    pub code: String,
    /// Expiry instant in RFC 3339.
    pub expires_at: String,
    /// Remaining seconds (computed server-side, indicative for the UI).
    pub seconds_remaining: i64,
}

/// Secret produced once a pairing code is validated — this is what
/// the extension stores in `chrome.storage.local` and presents with each
/// request.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingSecretDto {
    /// `id` of the created pairing (ULID).
    pub pairing_id: String,
    /// 32-byte secret, hex-encoded (64 chars). Passed as is to
    /// the extension through the bridge, then discarded by the desktop app.
    pub secret_hex: String,
}

/// UI representation of an active pairing (list in /parametres).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairingDto {
    pub id: String,
    pub fingerprint: String,
    pub created_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revoked_at: Option<String>,
}

impl From<&crate::extension_store::PairingRow> for PairingDto {
    fn from(r: &crate::extension_store::PairingRow) -> Self {
        Self {
            id: r.id.clone(),
            fingerprint: r.fingerprint.clone(),
            created_at: r.created_at.to_rfc3339(),
            last_seen_at: r.last_seen_at.map(|d| d.to_rfc3339()),
            revoked_at: r.revoked_at.map(|d| d.to_rfc3339()),
        }
    }
}

/// UI representation of an event ingested from the extension.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionEventDto {
    pub id: String,
    pub pairing_id: String,
    pub ts: String,
    pub method: String,
    pub model_id: String,
    pub tokens_in: u32,
    pub tokens_out: u32,
    pub gco2eq_p50: f64,
    pub water_ml: f64,
    pub energy_wh: f64,
    pub ingested_at: String,
}

impl From<&crate::extension_store::ExtensionEventRow> for ExtensionEventDto {
    fn from(r: &crate::extension_store::ExtensionEventRow) -> Self {
        Self {
            id: r.id.clone(),
            pairing_id: r.pairing_id.clone(),
            ts: r.ts.to_rfc3339(),
            method: r.method.clone(),
            model_id: r.model_id.clone(),
            tokens_in: r.tokens_in,
            tokens_out: r.tokens_out,
            gco2eq_p50: r.gco2eq_p50,
            water_ml: r.water_ml,
            energy_wh: r.energy_wh,
            ingested_at: r.ingested_at.to_rfc3339(),
        }
    }
}

/// Fields extracted from a ledger `estimation_result_json` payload.
/// Used by [`AuditEntrySummaryDto::from_entry`].
struct ParsedPayload {
    model_id: String,
    co2eq_p50: f64,
    method: sobria_core::EmpreinteMethod,
}

fn parse_payload_full(payload: &str) -> ParsedPayload {
    if payload == sobria_audit::PURGED_SENTINEL {
        return ParsedPayload {
            model_id: "(purgé)".into(),
            co2eq_p50: f64::NAN,
            // A purged entry no longer reveals its method; default to
            // AfnorSobria (sentinel consistent with the historical v1 ledger).
            method: sobria_core::EmpreinteMethod::AfnorSobria,
        };
    }
    let parsed: Result<EstimationResult, _> = serde_json::from_str(payload);
    match parsed {
        Ok(r) => {
            let model_id = r.request.model_id.clone();
            let co2eq_p50 = r
                .indicators
                .iter()
                .find(|i| matches!(i.indicator, Indicator::Co2Eq))
                .map_or(f64::NAN, |i| i.interval.p50);
            ParsedPayload {
                model_id,
                co2eq_p50,
                method: r.method,
            }
        },
        Err(_) => ParsedPayload {
            model_id: "(invalide)".into(),
            co2eq_p50: f64::NAN,
            method: sobria_core::EmpreinteMethod::AfnorSobria,
        },
    }
}

/// Backward-compatible wrapper of [`parse_payload_full`] for legacy code that
/// only wants `(model_id, co2eq_p50)`.
///
/// Currently used only by tests — kept under `#[cfg(test)]` until
/// the final migration / removal.
#[cfg(test)]
fn parse_payload(payload: &str) -> (String, f64) {
    let p = parse_payload_full(payload);
    (p.model_id, p.co2eq_p50)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sobria_core::{EstimationRequest, Indicator, IndicatorValue, UncertaintyInterval};

    #[test]
    fn estimation_request_dto_round_trip_json() {
        let dto = EstimationRequestDto {
            model_id: "gpt-4o-mini".into(),
            tokens_in: 100,
            tokens_out_estimated: 500,
            datacenter_id: None,
            method: None,
            modalities: None,
            overhead: None,
        };
        let json = serde_json::to_string(&dto).unwrap();
        let back: EstimationRequestDto = serde_json::from_str(&json).unwrap();
        assert_eq!(back.model_id, "gpt-4o-mini");
        assert_eq!(back.tokens_in, 100);
    }

    #[test]
    fn parse_payload_purged_returns_sentinel() {
        let (id, co2) = parse_payload(sobria_audit::PURGED_SENTINEL);
        assert_eq!(id, "(purgé)");
        assert!(co2.is_nan());
    }

    #[test]
    fn parse_payload_invalid_returns_invalid_marker() {
        let (id, _co2) = parse_payload("{not valid json");
        assert_eq!(id, "(invalide)");
    }

    #[test]
    fn app_preferences_dto_round_trip_with_default_datacenter_id() {
        let dto = AppPreferencesDto {
            persona: None,
            enabled_modules: vec![],
            onboarded: false,
            lang: "fr".into(),
            default_method: sobria_core::EmpreinteMethod::AfnorSobria,
            also_show_methods: vec![],
            default_datacenter_id: Some("ovh-gra-gravelines".into()),
        };
        let json = serde_json::to_string(&dto).unwrap();
        assert!(json.contains("default_datacenter_id"));
        let back: AppPreferencesDto = serde_json::from_str(&json).unwrap();
        assert_eq!(
            back.default_datacenter_id.as_deref(),
            Some("ovh-gra-gravelines")
        );

        // Backward-compat: a JSON without the field must deserialize with None.
        let legacy = serde_json::json!({
            "persona": null,
            "enabled_modules": [],
            "onboarded": false,
            "lang": "fr",
            "default_method": "afnor_sobria",
            "also_show_methods": []
        });
        let parsed: AppPreferencesDto = serde_json::from_value(legacy).unwrap();
        assert!(parsed.default_datacenter_id.is_none());
    }

    #[test]
    fn parse_payload_valid_extracts_model_and_co2() {
        let result = EstimationResult {
            method: sobria_core::EmpreinteMethod::AfnorSobria,
            request: EstimationRequest {
                model_id: "claude-3-5-sonnet".into(),
                tokens_in: 50,
                tokens_out_estimated: 200,
                datacenter_id: None,
                timestamp: Utc::now(),
                modalities: Vec::new(),
                overhead: sobria_core::ContextOverhead::default(),
            },
            indicators: vec![IndicatorValue {
                indicator: Indicator::Co2Eq,
                interval: UncertaintyInterval::new(1.0, 2.5, 4.0).unwrap(),
                unit: "gCO2eq".into(),
                bins: None,
            }],
            equivalents: vec![],
            hypotheses: vec![],
            computed_at: Utc::now(),
            seed: 42,
        };
        let payload = serde_json::to_string(&result).unwrap();
        let (id, co2) = parse_payload(&payload);
        assert_eq!(id, "claude-3-5-sonnet");
        assert!((co2 - 2.5).abs() < 1e-9);
    }
}
