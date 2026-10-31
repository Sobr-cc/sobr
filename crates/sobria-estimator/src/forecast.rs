//! Checks and growth step shared by the two forecasts: `scenarios` (M13) and
//! `yearly_forecast` (M16). Each caller keeps its own order of checks.

use crate::error::{EstimatorError, EstimatorResult};

/// Largest `base_volume_per_day` accepted (prompts per day).
const MAX_VOLUME_PER_DAY: f64 = 1_000_000.0;

pub(crate) fn check_scenario_count(count: usize, max: usize) -> EstimatorResult<()> {
    if count > max {
        return Err(EstimatorError::Schema(format!(
            "trop de scénarios : {count} (max {max})"
        )));
    }
    Ok(())
}

pub(crate) fn check_months(months: u32, max: u32) -> EstimatorResult<()> {
    if months == 0 || months > max {
        return Err(EstimatorError::Schema(format!(
            "months {months} hors bornes [1, {max}]"
        )));
    }
    Ok(())
}

pub(crate) fn check_volume_per_day(volume: f64) -> EstimatorResult<()> {
    if !volume.is_finite() || volume < 0.0 || volume > MAX_VOLUME_PER_DAY {
        return Err(EstimatorError::Schema(format!(
            "base_volume_per_day {volume} hors bornes [0, 10⁶]"
        )));
    }
    Ok(())
}

pub(crate) fn duplicate_label(label: &str) -> EstimatorError {
    EstimatorError::Schema(format!("label scénario en doublon : '{label}'"))
}

/// `base × growth^n`, the value of month `n` (`n` is at most 60 once checked).
pub(crate) fn grown(base: f64, growth: f64, n: u32) -> f64 {
    base * growth.powi(i32::try_from(n).unwrap_or(i32::MAX))
}
