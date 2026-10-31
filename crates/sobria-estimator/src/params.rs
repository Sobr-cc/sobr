//! Distribution parameters of the calculation model.
//!
//! See CDC §9 and ADR-0004 for the reference formula.
//! See `docs/archive/chantiers/C05-estimator-monte-carlo.md` §4.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    distributions::Distribution,
    error::{EstimatorError, EstimatorResult},
};

/// PUE used when no datacenter is known. No source yet (SOB-209).
pub(crate) const DEFAULT_PUE: Distribution = Distribution::Uniform {
    low: 1.1,
    high: 1.4,
};

/// WUE in L/kWh used when no datacenter is known. No source yet (SOB-209).
pub(crate) const DEFAULT_WUE_L_PER_KWH: Distribution = Distribution::Uniform {
    low: 0.5,
    high: 2.5,
};

/// France 2024 electricity mix (ADEME, ~56 gCO₂eq/kWh), used when no
/// datacenter is known, whatever the model's hosting (SOB-209).
pub(crate) const FRANCE_MIX_G_PER_KWH: Distribution = Distribution::Point { value: 56.0 };

/// Distributions of the physical parameters of an estimation.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq)]
pub struct EstimationParams {
    /// Prefill (input) energy per token, in milli-joules per token.
    pub epsilon_prefill_mj_per_token: Distribution,
    /// Decode (output) energy per token, in milli-joules per token.
    pub epsilon_decode_mj_per_token: Distribution,
    /// Datacenter PUE (unitless, typically between 1.05 and 1.6).
    pub pue: Distribution,
    /// Electricity emission factor, in gCO₂eq/kWh.
    pub if_electrical_g_per_kwh: Distribution,
    /// Embodied carbon amortised per request, in gCO₂eq.
    pub embodied_g_per_request: Distribution,
    /// Water Usage Effectiveness, in litres per IT kWh.
    pub wue_l_per_kwh: Distribution,
}

impl EstimationParams {
    /// Builds a conservative "default" parameter set — reasonable values
    /// when no specific data is available. Use only in tests and the
    /// demo.
    ///
    /// Indicative sources for the values:
    /// - ε_prefill / ε_decode: order of magnitude from HF AI Energy Score 2026.
    /// - PUE: modern datacenter, range 1.1-1.4.
    /// - Electricity IF: France 2024 mix (ADEME ~56 g/kWh).
    /// - Embodied / req: Gupta 2022, amortised over ~1 G req.
    /// - WUE: Mytton 2021, mid range.
    #[must_use]
    pub fn conservative_default() -> Self {
        Self {
            // 1 mJ/token median, ±50 %
            epsilon_prefill_mj_per_token: Distribution::LogNormal {
                mu: (1.0_f64).ln(),
                sigma: 0.25,
            },
            // 2 mJ/token median (decode costs more than prefill)
            epsilon_decode_mj_per_token: Distribution::LogNormal {
                mu: (2.0_f64).ln(),
                sigma: 0.3,
            },
            pue: DEFAULT_PUE,
            if_electrical_g_per_kwh: FRANCE_MIX_G_PER_KWH,
            embodied_g_per_request: Distribution::LogNormal {
                mu: (0.02_f64).ln(),
                sigma: 0.4,
            },
            wue_l_per_kwh: DEFAULT_WUE_L_PER_KWH,
        }
    }

    /// Checks that all distributions are valid.
    pub fn validate(&self) -> EstimatorResult<()> {
        self.epsilon_prefill_mj_per_token
            .validate()
            .map_err(|e| EstimatorError::Validation(format!("epsilon_prefill: {e}")))?;
        self.epsilon_decode_mj_per_token
            .validate()
            .map_err(|e| EstimatorError::Validation(format!("epsilon_decode: {e}")))?;
        self.pue
            .validate()
            .map_err(|e| EstimatorError::Validation(format!("pue: {e}")))?;
        self.if_electrical_g_per_kwh
            .validate()
            .map_err(|e| EstimatorError::Validation(format!("if_electrical: {e}")))?;
        self.embodied_g_per_request
            .validate()
            .map_err(|e| EstimatorError::Validation(format!("embodied: {e}")))?;
        self.wue_l_per_kwh
            .validate()
            .map_err(|e| EstimatorError::Validation(format!("wue: {e}")))?;
        Ok(())
    }

    /// Fluent builder: replaces `pue`.
    #[must_use]
    pub fn with_pue(mut self, d: Distribution) -> Self {
        self.pue = d;
        self
    }

    /// Fluent builder: replaces `if_electrical_g_per_kwh`.
    #[must_use]
    pub fn with_if_electrical(mut self, d: Distribution) -> Self {
        self.if_electrical_g_per_kwh = d;
        self
    }

    /// Fluent builder: replaces `epsilon_decode_mj_per_token`.
    #[must_use]
    pub fn with_epsilon_decode(mut self, d: Distribution) -> Self {
        self.epsilon_decode_mj_per_token = d;
        self
    }

    /// Fluent builder: replaces `embodied_g_per_request`.
    #[must_use]
    pub fn with_embodied(mut self, d: Distribution) -> Self {
        self.embodied_g_per_request = d;
        self
    }

    /// Fluent builder: replaces `wue_l_per_kwh`.
    #[must_use]
    pub fn with_wue(mut self, d: Distribution) -> Self {
        self.wue_l_per_kwh = d;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conservative_default_is_valid() {
        let p = EstimationParams::conservative_default();
        assert!(p.validate().is_ok());
    }

    #[test]
    fn builder_chains() {
        let p = EstimationParams::conservative_default()
            .with_pue(Distribution::Point { value: 1.3 })
            .with_if_electrical(Distribution::Point { value: 412.0 });
        assert_eq!(p.pue, Distribution::Point { value: 1.3 });
        assert_eq!(
            p.if_electrical_g_per_kwh,
            Distribution::Point { value: 412.0 }
        );
        // The other fields are left unchanged
        assert!(p.validate().is_ok());
    }

    #[test]
    fn validate_catches_bad_subparam() {
        let p = EstimationParams::conservative_default().with_pue(Distribution::Uniform {
            low: 5.0,
            high: 1.0,
        });
        let err = p.validate().unwrap_err();
        assert!(format!("{err}").contains("pue"));
    }

    #[test]
    fn serde_round_trip() {
        let p = EstimationParams::conservative_default();
        let json = serde_json::to_string(&p).unwrap();
        let back: EstimationParams = serde_json::from_str(&json).unwrap();
        assert_eq!(back, p);
    }
}
