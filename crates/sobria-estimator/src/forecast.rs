//! Checks and growth step shared by the two forecasts: `scenarios` (M13) and
//! `yearly_forecast` (M16). Each caller keeps its own order of checks.

use crate::error::{EstimatorError, EstimatorResult};

/// Largest `base_volume_per_day` accepted (prompts per day).
const MAX_VOLUME_PER_DAY: f64 = 1_000_000.0;

/// Days counted in one forecast month.
pub(crate) const DAYS_PER_MONTH: f64 = 30.0;

pub(crate) fn growth_in_bounds(monthly_growth_pct: f64) -> bool {
    (-50.0..=50.0).contains(&monthly_growth_pct)
}

/// Monthly multiplier for a growth in percent (`5.0` → `1.05`).
pub(crate) fn growth_factor(monthly_growth_pct: f64) -> f64 {
    1.0 + monthly_growth_pct / 100.0
}

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn months_must_be_between_1_and_the_maximum() {
        assert!(check_months(0, 60).is_err());
        assert!(check_months(1, 60).is_ok());
        assert!(check_months(60, 60).is_ok());
        assert!(check_months(61, 60).is_err());
    }

    #[test]
    fn volume_must_be_finite_and_between_0_and_10_6() {
        for ok in [0.0, 1.0, MAX_VOLUME_PER_DAY] {
            assert!(check_volume_per_day(ok).is_ok(), "{ok}");
        }
        for bad in [-1.0, MAX_VOLUME_PER_DAY + 1.0, f64::NAN, f64::INFINITY] {
            assert!(check_volume_per_day(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn growth_is_bounded_to_plus_or_minus_50_pct() {
        assert!(growth_in_bounds(-50.0));
        assert!(growth_in_bounds(50.0));
        assert!(!growth_in_bounds(-50.1));
        assert!(!growth_in_bounds(50.1));
        assert!(!growth_in_bounds(f64::NAN));
    }

    #[test]
    fn scenario_count_is_capped() {
        assert!(check_scenario_count(10, 10).is_ok());
        assert!(check_scenario_count(11, 10).is_err());
    }

    #[test]
    fn grown_compounds_the_monthly_factor() {
        assert_eq!(grown(100.0, growth_factor(10.0), 0), 100.0);
        assert!((grown(100.0, growth_factor(10.0), 2) - 121.0).abs() < 1e-9);
        assert!((grown(100.0, growth_factor(-50.0), 1) - 50.0).abs() < 1e-9);
    }
}
