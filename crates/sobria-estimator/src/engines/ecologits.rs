//! **EcoLogits** engine: a direct port of the formulas published in:
//!
//! - Rincé S., Banse A., *EcoLogits: Evaluating the Environmental Impacts
//!   of Generative AI*, JOSS 10(111):7471, 2025.
//!   [doi:10.21105/joss.07471](https://doi.org/10.21105/joss.07471)
//! - Official documentation:
//!   <https://ecologits.ai/latest/methodology/llm_inference/>
//! - Reference source code (CC BY-SA 4.0):
//!   <https://github.com/genai-impact/ecologits>
//!
//! ## License & attribution
//!
//! The **formulas** ported here are published by GenAI Impact under
//! **CC BY-SA 4.0** ([text](https://creativecommons.org/licenses/by-sa/4.0/)).
//! This Rust port is under the MIT licence, like the rest of Sobr.ia. The
//! CC BY-SA attribution is given by the references above and by the
//! methodology card shown in the app (`/methodologies`); a fuller attribution
//! document is SOB-209.
//!
//! ## Differences from the Python reference implementation
//!
//! This v1.0 of the port:
//! - **Is deterministic**: it does not sample the internal coefficients
//!   (`α`, `β`, `γ`) but takes their published point values. A user who
//!   wants a P5/P95 range uses `AfnorSobria` instead. (EcoLogits with
//!   uncertainty: v1.1+.)
//! - **Uses B=64** (vLLM, typical cloud batch), as a constant.
//! - **Uses an H100 80GB / p5.48xlarge server** as the reference.
//! - **Uses only** `pue`, `if_electrical_g_per_kwh` and `wue_l_per_kwh` from
//!   `EstimationParams` (`epsilon_*` and `embodied_g_per_request` are
//!   *ignored*, replaced by the internal EcoLogits formulas).

use chrono::Utc;
use sobria_core::{EstimationRequest, EstimationResult, Hypothesis, UncertaintyInterval};

use crate::{
    distributions::Distribution,
    effective_tokens,
    engine_trait::{distribution_hypothesis, indicator_values, EmpreinteEngine, EmpreinteMethod},
    equivalents,
    error::{EstimatorError, EstimatorResult},
    model_presets::find_preset,
    params::EstimationParams,
};

// ─────────────────────────────────────────────────────────────────────────────
// EcoLogits 2026-01 constants
// Source: <https://ecologits.ai/latest/methodology/llm_inference/>
// Accessed and pinned to version 2026-01 (see notebook/validation.qmd)
// ─────────────────────────────────────────────────────────────────────────────

/// `α` of the f_E fit (GPU energy per output token, Wh).
const ALPHA_E: f64 = 1.17e-6;
/// `β` of the f_E fit.
const BETA_E: f64 = -1.12e-2;
/// `γ` of the f_E fit.
const GAMMA_E: f64 = 4.05e-5;

/// `α` of the f_L fit (latency per token, s).
const ALPHA_L: f64 = 6.78e-4;
/// `β` of the f_L fit.
const BETA_L: f64 = 3.12e-4;
/// `γ` of the f_L fit.
const GAMMA_L: f64 = 1.94e-2;

/// Reference batch size (vLLM continuous batching).
const BATCH_SIZE: f64 = 64.0;

/// Electrical power of the reference server (excluding GPUs), p5.48xlarge.
const P_SERVER_W: f64 = 1200.0;

/// Number of GPUs installed in the reference server.
const N_GPU_INSTALLED: f64 = 8.0;

/// VRAM per GPU (H100 80 GB).
const MEM_GPU_GB: f64 = 80.0;

/// Bits per model weight (FP16).
const Q_BITS: f64 = 16.0;

/// Inference memory overhead (KV cache, activations, margins).
const MEM_OVERHEAD: f64 = 1.2;

/// Embodied carbon of the server excluding GPUs (Boavizta, amortised over 3 years).
const I_SERVER_NO_GPU_KG: f64 = 5700.0;

/// Embodied carbon of one H100 GPU (Boavizta).
const I_GPU_KG: f64 = 273.0;

/// Hardware lifetime (3 years, in seconds).
const HW_LIFETIME_SEC: f64 = 3.0 * 365.25 * 86_400.0;

// ─────────────────────────────────────────────────────────────────────────────
// EcoLogits formulas
// ─────────────────────────────────────────────────────────────────────────────

/// Number of GPUs needed to serve a model of `p_billions` billion
/// parameters in FP16, rounded up to the next power of 2.
///
/// Formula: `n_GPU = next_pow2(ceil(1.2 × P × 16/8 / 80))`.
#[must_use]
pub fn n_gpu(p_billions: f64) -> u32 {
    let m_model_gb = MEM_OVERHEAD * p_billions * Q_BITS / 8.0;
    let raw = (m_model_gb / MEM_GPU_GB).ceil() as u32;
    if raw <= 1 {
        1
    } else {
        raw.next_power_of_two()
    }
}

/// GPU energy per output token, Wh, at batch B=64.
///
/// `f_E(P, 64) = α × exp(β × 64) × P + γ`
#[must_use]
pub fn f_energy_per_token_wh(p_billions: f64) -> f64 {
    ALPHA_E * (BETA_E * BATCH_SIZE).exp() * p_billions + GAMMA_E
}

/// Latency per output token, seconds, at batch B=64.
///
/// `f_L(P, 64) = α' × P + β' × 64 + γ'`
#[must_use]
pub fn f_latency_per_token_sec(p_billions: f64) -> f64 {
    ALPHA_L * p_billions + BETA_L * BATCH_SIZE + GAMMA_L
}

/// Total energy of one request in kWh (usage only, excluding embodied).
#[must_use]
pub fn request_energy_kwh(p_billions: f64, tokens_out: u32, pue: f64) -> f64 {
    let n_gpu_f = f64::from(n_gpu(p_billions));
    let t_out = f64::from(tokens_out);
    let dt_sec = t_out * f_latency_per_token_sec(p_billions);
    let e_gpu_wh = n_gpu_f * t_out * f_energy_per_token_wh(p_billions);
    let e_server_no_gpu_wh =
        dt_sec * P_SERVER_W * (n_gpu_f / N_GPU_INSTALLED) / BATCH_SIZE / 3600.0;
    let e_server_wh = e_gpu_wh + e_server_no_gpu_wh;
    let e_request_wh = pue * e_server_wh;
    e_request_wh / 1000.0
}

/// Amortised embodied carbon per request (g CO₂eq).
///
/// `I_request_emb = (ΔT / (B × ΔL)) × I_server`
/// with `I_server = (n_GPU/N_installed) × I_server_noGPU + n_GPU × I_GPU`.
#[must_use]
pub fn request_embodied_co2eq_g(p_billions: f64, tokens_out: u32) -> f64 {
    let n_gpu_f = f64::from(n_gpu(p_billions));
    let t_out = f64::from(tokens_out);
    let dt_sec = t_out * f_latency_per_token_sec(p_billions);
    // kg CO2eq amortised for the request, then × 1000 to convert to g.
    let i_server_kg = (n_gpu_f / N_GPU_INSTALLED) * I_SERVER_NO_GPU_KG + n_gpu_f * I_GPU_KG;
    let alloc_ratio = dt_sec / (BATCH_SIZE * HW_LIFETIME_SEC);
    alloc_ratio * i_server_kg * 1000.0
}

// ─────────────────────────────────────────────────────────────────────────────
// Engine
// ─────────────────────────────────────────────────────────────────────────────

/// EcoLogits estimation engine (deterministic).
#[derive(Debug, Clone, Copy)]
pub struct EcoLogitsEngine {
    /// Seed kept for interface consistency with [`crate::engine::MonteCarloEngine`].
    /// Unused by the (deterministic) EcoLogits formulas, but recorded
    /// in the result for traceability.
    seed: u64,
}

impl EcoLogitsEngine {
    /// Builds an EcoLogits engine with the given seed (used only for
    /// traceability in the ledger).
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { seed }
    }

    /// Returns the configured seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Picks a representative value of a distribution (its median), used
    /// for PUE, IF and WUE, which `EstimationParams` may give as
    /// distributions.
    fn point_value(dist: Distribution) -> f64 {
        match dist {
            Distribution::Point { value } => value,
            Distribution::Uniform { low, high } => f64::midpoint(low, high),
            Distribution::Normal { mean, .. } => mean,
            Distribution::LogNormal { mu, .. } => mu.exp(),
        }
    }

    /// Runs the EcoLogits estimation.
    ///
    /// `params.pue`, `params.if_electrical_g_per_kwh` and `params.wue_l_per_kwh`
    /// are used (reduced to their central value when they are distributions).
    /// `params.epsilon_*` and `params.embodied_g_per_request` are **ignored**,
    /// replaced by the internal EcoLogits formulas.
    // Linear pipeline (validation → params → energy → CO2 → water →
    // indicators/equivalents/hypotheses → result). Most of the length comes
    // from the explicit `Hypothesis` literals meant for the audit; splitting
    // into helpers would lose cohesion without gaining clarity.
    #[allow(clippy::too_many_lines)]
    pub fn estimate(
        &self,
        request: &EstimationRequest,
        params: &EstimationParams,
    ) -> EstimatorResult<EstimationResult> {
        request
            .validate()
            .map_err(|e| EstimatorError::Validation(format!("request: {e}")))?;
        params.validate()?;

        // Look the model up in the registry to get its parameter count
        // in billions.
        let preset = find_preset(&request.model_id).ok_or_else(|| {
            EstimatorError::Schema(format!(
                "EcoLogits: modèle inconnu {:?} (registry Sobr.ia)",
                request.model_id
            ))
        })?;
        let p_billions = preset.approx_params_billions;

        let pue = Self::point_value(params.pue);
        let if_elec = Self::point_value(params.if_electrical_g_per_kwh);
        let wue = Self::point_value(params.wue_l_per_kwh);

        // Effective tokens. EcoLogits mainly uses t_out in its
        // f_energy_per_token / f_latency_per_token formulas, so we feed it
        // effective_out (which includes auto thinking tokens for reasoning
        // models). The input-side overhead (system + tools + memory
        // + modalities) does not count in the EcoLogits energy model (which
        // charges per decode token); this is kept for consistency with
        // the published methodology.
        let (_effective_in, effective_out) =
            effective_tokens::effective_tokens(request, Some(preset));

        let energy_kwh = request_energy_kwh(p_billions, effective_out, pue);
        let energy_watt_hours = energy_kwh * 1000.0;
        let usage_co2_g = energy_kwh * if_elec;

        // Embodied: the internal EcoLogits formula by default. But if the
        // user explicitly passed `params.embodied = Point(0)` (a "usage-only"
        // override), we respect that choice. Typical case: usage-only
        // ReproductionCase and cross-comparisons.
        let embodied_co2_g = match params.embodied_g_per_request {
            Distribution::Point { value: 0.0 } => 0.0,
            _ => request_embodied_co2eq_g(p_billions, effective_out),
        };

        let total_co2_g = usage_co2_g + embodied_co2_g;

        // Water (same formula as Sobr.ia: E_kWh × WUE)
        let water_l = energy_kwh * wue;

        // Output: EcoLogits is deterministic in v1, so P5 = P50 = P95.
        let degenerate = |v: f64| {
            UncertaintyInterval::point(v)
                .map_err(|e| EstimatorError::Validation(format!("interval EcoLogits ({v}): {e}")))
        };
        // No histogram: every sample would be the same value.
        let indicators = indicator_values(
            (degenerate(total_co2_g)?, None),
            (degenerate(energy_watt_hours)?, None),
            (degenerate(water_l)?, None),
        );
        let equivalents = equivalents::standard(total_co2_g, energy_watt_hours);

        // Hypotheses: record the EcoLogits coefficients + the user params
        // (PUE / IF / WUE) actually used.
        let hypotheses = vec![
            Hypothesis {
                key: "method".into(),
                value: serde_json::Value::String("ecologits-2026-01".into()),
                source: "EcoLogits — doi:10.21105/joss.07471 (CC BY-SA 4.0)".into(),
            },
            Hypothesis {
                key: "model_params_billions".into(),
                value: serde_json::Value::from(p_billions),
                source: format!("Sobr.ia preset {}", preset.id),
            },
            Hypothesis {
                key: "n_gpu".into(),
                value: serde_json::Value::from(n_gpu(p_billions)),
                source: "EcoLogits formula: next_pow2(ceil(1.2×P×16/8 / 80))".into(),
            },
            distribution_hypothesis(
                "pue",
                &params.pue,
                "User param (réduit à la médiane pour EcoLogits déterministe)",
            ),
            distribution_hypothesis(
                "if_electrical_g_per_kwh",
                &params.if_electrical_g_per_kwh,
                "User param (réduit à la médiane)",
            ),
            distribution_hypothesis(
                "wue_l_per_kwh",
                &params.wue_l_per_kwh,
                "User param (réduit à la médiane)",
            ),
        ];

        Ok(EstimationResult {
            method: EmpreinteMethod::EcoLogits,
            request: request.clone(),
            indicators,
            equivalents,
            hypotheses,
            computed_at: Utc::now(),
            seed: self.seed,
        })
    }
}

impl Default for EcoLogitsEngine {
    fn default() -> Self {
        Self::new(sobria_core::DEFAULT_SEED)
    }
}

impl EmpreinteEngine for EcoLogitsEngine {
    fn method(&self) -> EmpreinteMethod {
        EmpreinteMethod::EcoLogits
    }

    fn estimate(
        &self,
        request: &EstimationRequest,
        params: &EstimationParams,
    ) -> EstimatorResult<EstimationResult> {
        EcoLogitsEngine::estimate(self, request, params)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::validation::request_for as req;
    use sobria_core::Indicator;

    fn params_point(pue: f64, if_elec: f64, wue: f64) -> EstimationParams {
        EstimationParams::for_model("llama-3-1-70b")
            .unwrap()
            .with_pue(Distribution::Point { value: pue })
            .with_if_electrical(Distribution::Point { value: if_elec })
            .with_wue(Distribution::Point { value: wue })
    }

    #[test]
    fn n_gpu_llama_70b_is_4() {
        // 1.2 × 70 × 16/8 = 168 GB ; ceil(168/80) = 3 ; next_pow2(3) = 4.
        assert_eq!(n_gpu(70.0), 4);
    }

    #[test]
    fn n_gpu_llama_8b_is_1() {
        // 1.2 × 8 × 16/8 = 19.2 GB ; ceil(19.2/80) = 1.
        assert_eq!(n_gpu(8.0), 1);
    }

    #[test]
    fn n_gpu_mistral_123b_is_4() {
        // 1.2 × 123 × 16/8 = 295.2 GB ; ceil(295.2/80) = 4.
        assert_eq!(n_gpu(123.0), 4);
    }

    #[test]
    fn n_gpu_gpt_4o_200b_is_8() {
        // 1.2 × 200 × 16/8 = 480 GB ; ceil(480/80) = 6 ; next_pow2(6) = 8.
        assert_eq!(n_gpu(200.0), 8);
    }

    #[test]
    fn f_energy_at_b64_llama_70b() {
        // f_E(70, 64) = 1.17e-6 × exp(-0.7168) × 70 + 4.05e-5
        //            ≈ 4.00e-5 + 4.05e-5 ≈ 8.05e-5 Wh/token.
        let v = f_energy_per_token_wh(70.0);
        assert!((v - 8.05e-5).abs() < 1.0e-7, "f_E(70) ≈ 8.05e-5, reçu {v}");
    }

    #[test]
    fn f_latency_at_b64_llama_70b() {
        // f_L(70, 64) = 6.78e-4 × 70 + 3.12e-4 × 64 + 1.94e-2
        //            ≈ 0.0475 + 0.0200 + 0.0194 = 0.0869 s/token.
        let v = f_latency_per_token_sec(70.0);
        assert!((v - 0.0869).abs() < 1.0e-3, "f_L(70) ≈ 0.0869, reçu {v}");
    }

    #[test]
    fn llama_70b_fr_500_tokens_matches_reference() {
        // Target recomputed in Python (notebook/validation.qmd):
        //   E_request = 0.329 Wh, usage_g (FR 56) = 0.01843 g.
        let eng = EcoLogitsEngine::default();
        let res = eng
            .estimate(
                &req("llama-3-1-70b", 100, 500),
                &params_point(1.2, 56.0, 1.5),
            )
            .unwrap();
        let co2 = res
            .indicators
            .iter()
            .find(|i| i.indicator == Indicator::Co2Eq)
            .unwrap();
        let total = co2.interval.p50;
        let embodied = request_embodied_co2eq_g(70.0, 500);
        let usage = total - embodied;
        assert!(
            (usage - 0.01843).abs() / 0.01843 < 0.005,
            "usage {usage} ≠ 0.01843 ±0.5%"
        );
    }

    #[test]
    fn deterministic_intervals_are_degenerate() {
        let eng = EcoLogitsEngine::default();
        let res = eng
            .estimate(
                &req("llama-3-1-70b", 100, 500),
                &params_point(1.2, 56.0, 1.5),
            )
            .unwrap();
        for ind in &res.indicators {
            assert!(
                (ind.interval.p95 - ind.interval.p5).abs() < 1e-12,
                "EcoLogits déterministe → P5 = P95 attendu pour {:?}, reçu p5={} p95={}",
                ind.indicator,
                ind.interval.p5,
                ind.interval.p95,
            );
        }
    }

    #[test]
    fn unknown_model_id_errors() {
        let eng = EcoLogitsEngine::default();
        let err = eng
            .estimate(&req("modele-bidon", 10, 50), &params_point(1.2, 56.0, 1.5))
            .unwrap_err();
        let msg = format!("{err}");
        assert!(msg.contains("modèle inconnu"), "message inattendu : {msg}");
    }

    #[test]
    fn implements_empreinte_engine_trait() {
        let eng: Box<dyn EmpreinteEngine> = Box::new(EcoLogitsEngine::default());
        assert_eq!(eng.method(), EmpreinteMethod::EcoLogits);
        assert_eq!(eng.methodology_info().method, EmpreinteMethod::EcoLogits);
    }

    #[test]
    fn doubling_tokens_approximately_doubles_co2eq() {
        let eng = EcoLogitsEngine::default();
        let p = params_point(1.2, 56.0, 1.5);
        let r1 = eng.estimate(&req("llama-3-1-70b", 100, 500), &p).unwrap();
        let r2 = eng.estimate(&req("llama-3-1-70b", 100, 1000), &p).unwrap();
        let g1 = r1.indicators[0].interval.p50;
        let g2 = r2.indicators[0].interval.p50;
        let ratio = g2 / g1;
        // Not exactly 2x because of the γ overhead (constant term
        // per token, more prefill-like). But between 1.9 and 2.1.
        assert!(
            (1.85..=2.15).contains(&ratio),
            "ratio attendu ~2.0, reçu {ratio}"
        );
    }

    #[test]
    fn higher_carbon_grid_increases_co2eq() {
        let eng = EcoLogitsEngine::default();
        let r_fr = eng
            .estimate(
                &req("llama-3-1-70b", 100, 500),
                &params_point(1.2, 56.0, 1.5),
            )
            .unwrap();
        let r_va = eng
            .estimate(
                &req("llama-3-1-70b", 100, 500),
                &params_point(1.2, 412.0, 1.5),
            )
            .unwrap();
        let g_fr = r_fr.indicators[0].interval.p50;
        let g_va = r_va.indicators[0].interval.p50;
        assert!(g_va > g_fr, "US-VA ({g_va}) doit > FR ({g_fr})");
    }
}
