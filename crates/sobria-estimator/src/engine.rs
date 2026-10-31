//! Monte-Carlo engine of the estimator (ADR-0004).
//!
//! N=10⁴ draws by default, with a fixed seed (`DEFAULT_SEED`, 42; nothing
//! reads `SOBRIA_SEED` yet, SOB-84).
//! See CDC §9 and `docs/archive/chantiers/C05-estimator-monte-carlo.md`.

use chrono::Utc;
use rand_xoshiro::{rand_core::SeedableRng, Xoshiro256PlusPlus};
use sobria_core::{
    DistributionBins, EstimationRequest, EstimationResult, Hypothesis, UncertaintyInterval,
    DEFAULT_SEED,
};
use tracing::debug;

use crate::{
    effective_tokens,
    engine_trait::{distribution_hypothesis, indicator_values, EmpreinteEngine, EmpreinteMethod},
    equivalents,
    error::{EstimatorError, EstimatorResult},
    model_presets::find_preset,
    params::EstimationParams,
};

/// Default number of draws. See ADR-0004.
pub const DEFAULT_N: u32 = 10_000;

/// Default number of bins for `bin_samples()`. See C09 §3.2 (option A1).
///
/// 50 bins give enough resolution to tell apart the right tail
/// of a log-normal distribution while keeping the audit payload light
/// (~200 bytes per indicator).
pub const DEFAULT_BIN_COUNT: usize = 50;

/// Minimum number of samples to compute a histogram.
///
/// Below it (unit tests with a small N), we return `None` to avoid
/// meaningless histograms.
pub const MIN_SAMPLES_FOR_BINS: usize = 10;

/// Reusable Monte-Carlo engine.
#[derive(Debug, Clone, Copy)]
pub struct MonteCarloEngine {
    n: u32,
    seed: u64,
}

impl Default for MonteCarloEngine {
    fn default() -> Self {
        Self::new(DEFAULT_SEED)
    }
}

impl MonteCarloEngine {
    /// Builds an engine with the given seed and N=10⁴ by default.
    #[must_use]
    pub fn new(seed: u64) -> Self {
        Self { n: DEFAULT_N, seed }
    }

    /// Replaces the number of draws.
    #[must_use]
    pub fn with_n(mut self, n: u32) -> Self {
        self.n = n;
        self
    }

    /// Returns the configured seed.
    #[must_use]
    pub fn seed(&self) -> u64 {
        self.seed
    }

    /// Returns N (number of draws).
    #[must_use]
    pub fn n(&self) -> u32 {
        self.n
    }

    /// Runs the estimation for a request + a parameter set.
    pub fn estimate(
        &self,
        request: &EstimationRequest,
        params: &EstimationParams,
    ) -> EstimatorResult<EstimationResult> {
        request
            .validate()
            .map_err(|e| EstimatorError::Validation(format!("request: {e}")))?;
        params.validate()?;
        if self.n == 0 {
            return Err(EstimatorError::Schema(
                "N=0 — aucun tirage à effectuer".into(),
            ));
        }

        let n_usize = self.n as usize;
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(self.seed);

        let mut co2eq_samples = Vec::with_capacity(n_usize);
        let mut energy_samples = Vec::with_capacity(n_usize);
        let mut water_samples = Vec::with_capacity(n_usize);

        // Effective tokens, including system overhead + modalities
        // + auto thinking tokens for reasoning models. If the model is
        // unknown, fall back to the raw values (no overhead/modality).
        let preset_opt = find_preset(&request.model_id);
        let (effective_in, effective_out) = effective_tokens::effective_tokens(request, preset_opt);
        let t_in = f64::from(effective_in);
        let t_out = f64::from(effective_out);

        for _ in 0..self.n {
            let eps_prefill = params.epsilon_prefill_mj_per_token.sample(&mut rng);
            let eps_decode = params.epsilon_decode_mj_per_token.sample(&mut rng);
            let pue = params.pue.sample(&mut rng);
            let if_elec = params.if_electrical_g_per_kwh.sample(&mut rng);
            let embodied = params.embodied_g_per_request.sample(&mut rng);
            let wue = params.wue_l_per_kwh.sample(&mut rng);

            // E_compute in mJ, then Wh: 1 Wh = 3 600 J = 3 600 000 mJ.
            // So E_total_wh = (E_compute_mj × PUE) / 3 600 000.
            let e_compute_mj = t_in * eps_prefill + t_out * eps_decode;
            let e_total_wh = (e_compute_mj * pue) / 3_600_000.0;

            // CO₂eq (g) = (E_total / 1000) * IF (g/kWh) + embodied
            let co2eq_g = (e_total_wh / 1000.0) * if_elec + embodied;

            // Water (L) = (E_total / 1000) * WUE (L/kWh)
            let water_l = (e_total_wh / 1000.0) * wue;

            co2eq_samples.push(co2eq_g);
            energy_samples.push(e_total_wh);
            water_samples.push(water_l);
        }

        // Binning BEFORE the in-place sort done by quantile_interval. It comes
        // first because `bin_samples` does not need a sorted slice and
        // `quantile_interval` mutates the slice via sort_by.
        let co2eq_bins = bin_samples(&co2eq_samples, DEFAULT_BIN_COUNT);
        let energy_bins = bin_samples(&energy_samples, DEFAULT_BIN_COUNT);
        let water_bins = bin_samples(&water_samples, DEFAULT_BIN_COUNT);

        let co2eq_interval = quantile_interval(&mut co2eq_samples)?;
        let energy_interval = quantile_interval(&mut energy_samples)?;
        let water_interval = quantile_interval(&mut water_samples)?;

        let indicators = indicator_values(
            (co2eq_interval, co2eq_bins),
            (energy_interval, energy_bins),
            (water_interval, water_bins),
        );
        let equivalents = equivalents::standard(co2eq_interval.p50, energy_interval.p50);

        let hypotheses = hypotheses_from_params(params);

        debug!(
            seed = self.seed,
            n = self.n,
            co2eq_p50 = co2eq_interval.p50,
            "monte-carlo terminé"
        );

        Ok(EstimationResult {
            method: EmpreinteMethod::AfnorSobria,
            request: request.clone(),
            indicators,
            equivalents,
            hypotheses,
            computed_at: Utc::now(),
            seed: self.seed,
        })
    }
}

/// Implementation of the common [`EmpreinteEngine`] trait for the AFNOR
/// SPEC 2314 Monte-Carlo engine (Sobr.ia's original methodology).
impl EmpreinteEngine for MonteCarloEngine {
    fn method(&self) -> EmpreinteMethod {
        EmpreinteMethod::AfnorSobria
    }

    fn estimate(
        &self,
        request: &EstimationRequest,
        params: &EstimationParams,
    ) -> EstimatorResult<EstimationResult> {
        MonteCarloEngine::estimate(self, request, params)
    }
}

/// Turns a list of draws into a P5/P50/P95 interval.
/// The list is sorted in place (mutable) to compute the quantiles.
fn quantile_interval(samples: &mut [f64]) -> EstimatorResult<UncertaintyInterval> {
    if samples.is_empty() {
        return Err(EstimatorError::Schema("samples vide".into()));
    }
    samples.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p5 = percentile(samples, 0.05);
    let p50 = percentile(samples, 0.50);
    let p95 = percentile(samples, 0.95);
    UncertaintyInterval::new(p5, p50, p95)
        .map_err(|e| EstimatorError::Validation(format!("interval: {e}")))
}

/// Returns the percentile of a **sorted** array. Simple linear
/// implementation, good enough for N=10⁴.
fn percentile(sorted: &[f64], p: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let idx_f = p * (sorted.len() as f64 - 1.0);
    let lo = idx_f.floor() as usize;
    let hi = idx_f.ceil() as usize;
    let frac = idx_f - lo as f64;
    if hi >= sorted.len() {
        sorted[sorted.len() - 1]
    } else {
        sorted[lo] * (1.0 - frac) + sorted[hi] * frac
    }
}

/// Builds an equal-width histogram of `n_bins` bins from a slice
/// of Monte-Carlo samples.
///
/// Returns `None` if:
/// - `samples.len() < MIN_SAMPLES_FOR_BINS` (typically tests with a small N),
/// - `n_bins == 0`,
/// - `min == max` (degenerate distribution, all values identical),
/// - a value is not finite (NaN or infinite).
///
/// Otherwise, returns `Some(DistributionBins { min, max, counts })` where:
/// - `min` = smallest observed value,
/// - `max` = largest observed value,
/// - `counts[i]` = number of samples in `[min + i·step, min + (i+1)·step)`,
///   except the last bin, which includes `max` (right-closed interval).
///
/// Guarantee: `counts.iter().sum::<u32>() as usize == samples.len()`.
#[must_use]
pub fn bin_samples(samples: &[f64], n_bins: usize) -> Option<DistributionBins> {
    if samples.len() < MIN_SAMPLES_FOR_BINS || n_bins == 0 {
        return None;
    }
    let mut min = f64::INFINITY;
    let mut max = f64::NEG_INFINITY;
    for &x in samples {
        if !x.is_finite() {
            return None;
        }
        if x < min {
            min = x;
        }
        if x > max {
            max = x;
        }
    }
    if (max - min).abs() < f64::EPSILON {
        return None;
    }
    let step = (max - min) / n_bins as f64;
    let mut counts = vec![0_u32; n_bins];
    for &x in samples {
        // Index = floor((x - min) / step), capped at n_bins - 1 (`max` falls
        // exactly on the upper bound → it goes into the last bin).
        let idx = ((x - min) / step).floor() as usize;
        let idx = idx.min(n_bins - 1);
        counts[idx] = counts[idx].saturating_add(1);
    }
    Some(DistributionBins { min, max, counts })
}

/// Builds the list of hypotheses from the distribution parameters.
/// One line per parameter, value serialised as JSON.
fn hypotheses_from_params(params: &EstimationParams) -> Vec<Hypothesis> {
    fn h(key: &str, dist: &crate::distributions::Distribution) -> Hypothesis {
        distribution_hypothesis(
            key,
            dist,
            "Sobr.ia — paramètres distributionnels (voir docs/methodology/)",
        )
    }
    vec![
        h(
            "epsilon_prefill_mj_per_token",
            &params.epsilon_prefill_mj_per_token,
        ),
        h(
            "epsilon_decode_mj_per_token",
            &params.epsilon_decode_mj_per_token,
        ),
        h("pue", &params.pue),
        h("if_electrical_g_per_kwh", &params.if_electrical_g_per_kwh),
        h("embodied_g_per_request", &params.embodied_g_per_request),
        h("wue_l_per_kwh", &params.wue_l_per_kwh),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::distributions::Distribution;
    use proptest::prelude::*;

    fn sample_request(tokens_in: u32, tokens_out: u32) -> EstimationRequest {
        crate::validation::request_for("test-model", tokens_in, tokens_out)
    }

    #[test]
    fn estimate_basic_case() {
        let engine = MonteCarloEngine::default();
        let params = EstimationParams::conservative_default();
        let req = sample_request(100, 500);
        let res = engine.estimate(&req, &params).unwrap();
        assert_eq!(res.indicators.len(), 3);
        assert_eq!(res.seed, DEFAULT_SEED);
        // All 3 indicators have strictly positive values
        for ind in &res.indicators {
            assert!(
                ind.interval.p50 > 0.0,
                "indicateur {:?} p50 ≤ 0",
                ind.indicator
            );
        }
    }

    #[test]
    fn reproducibility_same_seed_same_result() {
        let engine = MonteCarloEngine::new(123);
        let params = EstimationParams::conservative_default();
        let req = sample_request(50, 200);
        let r1 = engine.estimate(&req, &params).unwrap();
        let r2 = engine.estimate(&req, &params).unwrap();
        for (a, b) in r1.indicators.iter().zip(r2.indicators.iter()) {
            assert_eq!(
                a.interval, b.interval,
                "reproductibilité violée pour {:?}",
                a.indicator
            );
        }
    }

    #[test]
    fn quantile_order_preserved() {
        let engine = MonteCarloEngine::default();
        let params = EstimationParams::conservative_default();
        let req = sample_request(80, 300);
        let res = engine.estimate(&req, &params).unwrap();
        for ind in &res.indicators {
            assert!(ind.interval.p5 <= ind.interval.p50);
            assert!(ind.interval.p50 <= ind.interval.p95);
        }
    }

    #[test]
    fn doubling_tokens_approximately_doubles_co2eq() {
        // PUE + IF + ε fixed so only the token effect is measured
        let params = EstimationParams::conservative_default()
            .with_pue(Distribution::Point { value: 1.3 })
            .with_if_electrical(Distribution::Point { value: 56.0 })
            .with_embodied(Distribution::Point { value: 0.0 });
        let engine = MonteCarloEngine::new(42);
        let r1 = engine.estimate(&sample_request(100, 500), &params).unwrap();
        let r2 = engine
            .estimate(&sample_request(200, 1000), &params)
            .unwrap();
        let p50_1 = r1.indicators[0].interval.p50;
        let p50_2 = r2.indicators[0].interval.p50;
        let ratio = p50_2 / p50_1;
        assert!((ratio - 2.0).abs() < 0.05, "ratio ≠ 2 : {ratio}");
    }

    #[test]
    fn all_point_distributions_yield_degenerate_interval() {
        let params = EstimationParams {
            epsilon_prefill_mj_per_token: Distribution::Point { value: 1.0 },
            epsilon_decode_mj_per_token: Distribution::Point { value: 2.0 },
            pue: Distribution::Point { value: 1.3 },
            if_electrical_g_per_kwh: Distribution::Point { value: 56.0 },
            embodied_g_per_request: Distribution::Point { value: 0.02 },
            wue_l_per_kwh: Distribution::Point { value: 1.5 },
        };
        let engine = MonteCarloEngine::new(42).with_n(1000);
        let res = engine.estimate(&sample_request(100, 500), &params).unwrap();
        for ind in &res.indicators {
            // With only Point distributions, every draw is identical.
            assert!(
                (ind.interval.p95 - ind.interval.p5).abs() < 1e-9,
                "indicateur {:?} pas dégénéré (p5={}, p95={})",
                ind.indicator,
                ind.interval.p5,
                ind.interval.p95,
            );
        }
    }

    #[test]
    fn rejects_empty_model_id() {
        let mut req = sample_request(10, 50);
        req.model_id.clear();
        let res =
            MonteCarloEngine::default().estimate(&req, &EstimationParams::conservative_default());
        assert!(res.is_err());
    }

    #[test]
    fn rejects_n_zero() {
        let engine = MonteCarloEngine::default().with_n(0);
        let res = engine.estimate(
            &sample_request(10, 50),
            &EstimationParams::conservative_default(),
        );
        assert!(res.is_err());
    }

    #[test]
    fn result_contains_hypotheses_and_equivalents() {
        let res = MonteCarloEngine::default()
            .estimate(
                &sample_request(50, 250),
                &EstimationParams::conservative_default(),
            )
            .unwrap();
        assert_eq!(res.hypotheses.len(), 6, "6 hypothèses (1 par paramètre)");
        assert_eq!(res.equivalents.len(), 3);
    }

    proptest! {
        /// For every (valid request, conservative params) pair, the result
        /// is valid (3 indicators, ordered intervals, finite values).
        #[test]
        fn prop_result_is_valid(
            tokens_in in 1u32..10_000,
            tokens_out in 1u32..10_000,
            seed in 0u64..1_000,
        ) {
            let engine = MonteCarloEngine::new(seed).with_n(500);
            let params = EstimationParams::conservative_default();
            let req = sample_request(tokens_in, tokens_out);
            let res = engine.estimate(&req, &params).unwrap();
            prop_assert_eq!(res.indicators.len(), 3);
            for ind in &res.indicators {
                prop_assert!(ind.interval.p5.is_finite());
                prop_assert!(ind.interval.p50.is_finite());
                prop_assert!(ind.interval.p95.is_finite());
                prop_assert!(ind.interval.p5 <= ind.interval.p50);
                prop_assert!(ind.interval.p50 <= ind.interval.p95);
            }
        }
    }

    // ─────────────────────────────────────────────────────────────────────
    // bin_samples + integration into estimate()
    // ─────────────────────────────────────────────────────────────────────

    #[test]
    fn bin_samples_classic_case_sums_to_n() {
        // Uniform draws in [0, 1] via deterministic Xoshiro
        let mut rng = Xoshiro256PlusPlus::seed_from_u64(7);
        let samples: Vec<f64> = (0..10_000)
            .map(|_| {
                use rand_xoshiro::rand_core::RngCore;
                rng.next_u64() as f64 / u64::MAX as f64
            })
            .collect();
        let bins = bin_samples(&samples, DEFAULT_BIN_COUNT).unwrap();
        assert_eq!(bins.counts.len(), DEFAULT_BIN_COUNT);
        let total: u64 = bins.counts.iter().map(|&c| u64::from(c)).sum();
        assert_eq!(total, 10_000, "counts.sum() doit égaler N");
        assert!(bins.min < bins.max);
        assert!(bins.min >= 0.0);
        assert!(bins.max <= 1.0);
    }

    #[test]
    fn bin_samples_too_few_returns_none() {
        let samples = vec![1.0, 2.0, 3.0];
        assert!(bin_samples(&samples, DEFAULT_BIN_COUNT).is_none());
    }

    #[test]
    fn bin_samples_all_identical_returns_none() {
        let samples = vec![42.0; 10_000];
        assert!(
            bin_samples(&samples, DEFAULT_BIN_COUNT).is_none(),
            "min==max doit retourner None"
        );
    }

    #[test]
    fn bin_samples_zero_bins_returns_none() {
        let samples = vec![1.0; 100];
        assert!(bin_samples(&samples, 0).is_none());
    }

    #[test]
    fn bin_samples_rejects_nan() {
        let samples = vec![1.0, 2.0, f64::NAN, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0];
        assert!(bin_samples(&samples, 50).is_none());
    }

    #[test]
    fn estimate_attaches_bins_to_all_indicators() {
        let engine = MonteCarloEngine::default();
        let params = EstimationParams::conservative_default();
        let res = engine.estimate(&sample_request(100, 500), &params).unwrap();
        for ind in &res.indicators {
            let bins = ind
                .bins
                .as_ref()
                .unwrap_or_else(|| panic!("bins manquant pour {:?}", ind.indicator));
            assert_eq!(bins.counts.len(), DEFAULT_BIN_COUNT);
            let total: u64 = bins.counts.iter().map(|&c| u64::from(c)).sum();
            assert_eq!(
                total,
                u64::from(DEFAULT_N),
                "counts.sum() doit égaler N pour {:?}",
                ind.indicator
            );
            // Consistency with the quantile interval: min ≤ p5, p95 ≤ max
            // (within epsilon, since quantile interpolates linearly).
            assert!(
                bins.min <= ind.interval.p5 + 1e-9,
                "bins.min ({}) > p5 ({}) pour {:?}",
                bins.min,
                ind.interval.p5,
                ind.indicator
            );
            assert!(
                ind.interval.p95 - 1e-9 <= bins.max,
                "p95 ({}) > bins.max ({}) pour {:?}",
                ind.interval.p95,
                bins.max,
                ind.indicator
            );
        }
    }

    #[test]
    fn estimate_with_small_n_skips_bins() {
        // N=5 < MIN_SAMPLES_FOR_BINS → bins == None for every indicator
        let engine = MonteCarloEngine::default().with_n(5);
        let params = EstimationParams::conservative_default();
        let res = engine.estimate(&sample_request(10, 20), &params).unwrap();
        for ind in &res.indicators {
            assert!(
                ind.bins.is_none(),
                "bins doit être None pour N<10 ({:?})",
                ind.indicator
            );
        }
    }

    #[test]
    fn estimate_point_distributions_produces_no_bins() {
        // Every distribution is Point → all samples identical → bins=None
        let params = EstimationParams {
            epsilon_prefill_mj_per_token: Distribution::Point { value: 1.0 },
            epsilon_decode_mj_per_token: Distribution::Point { value: 2.0 },
            pue: Distribution::Point { value: 1.3 },
            if_electrical_g_per_kwh: Distribution::Point { value: 56.0 },
            embodied_g_per_request: Distribution::Point { value: 0.02 },
            wue_l_per_kwh: Distribution::Point { value: 1.5 },
        };
        let engine = MonteCarloEngine::new(42);
        let res = engine.estimate(&sample_request(100, 500), &params).unwrap();
        for ind in &res.indicators {
            assert!(
                ind.bins.is_none(),
                "Point distributions → bins None ({:?})",
                ind.indicator
            );
        }
    }
}
