//! Time aggregates of the audit ledger (M15 / C19).
//!
//! Pure module: takes a `&[AuditEntry]` and produces a `DashboardSummary`
//! for a given period. No I/O — the caller (`logic`) pulls the entries
//! from the ledger.

use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};
use serde::{Deserialize, Serialize};
use sobria_audit::AuditEntry;
use sobria_core::{EstimationResult, Indicator};

/// Dashboard periods supported in v1.0.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardPeriod {
    Today,
    Last7Days,
    ThisMonth,
    LastMonth,
    ThisYear,
}

impl DashboardPeriod {
    /// Parses a string (used by the IPC commands).
    #[must_use]
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "today" => Some(Self::Today),
            "last_7_days" => Some(Self::Last7Days),
            "this_month" => Some(Self::ThisMonth),
            "last_month" => Some(Self::LastMonth),
            "this_year" => Some(Self::ThisYear),
            _ => None,
        }
    }

    /// Human-readable display label.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            Self::Today => "Aujourd'hui",
            Self::Last7Days => "7 derniers jours",
            Self::ThisMonth => "Ce mois-ci",
            Self::LastMonth => "Mois précédent",
            Self::ThisYear => "Cette année",
        }
    }

    /// Computes the `[start, end)` window for this period, anchored at `now`.
    #[must_use]
    pub fn window(self, now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        match self {
            Self::Today => (start_of_day(now), now),
            Self::Last7Days => (now - Duration::days(7), now),
            Self::ThisMonth => (start_of_month(now), now),
            Self::LastMonth => {
                let this_start = start_of_month(now);
                let last_start = start_of_previous_month(this_start);
                (last_start, this_start)
            },
            Self::ThisYear => (start_of_year(now), now),
        }
    }

    /// **Previous** window for comparison (the period before the current one).
    #[must_use]
    pub fn previous_window(self, now: DateTime<Utc>) -> (DateTime<Utc>, DateTime<Utc>) {
        let (start, end) = self.window(now);
        match self {
            Self::Today => (start - Duration::days(1), end - Duration::days(1)),
            Self::Last7Days => (start - Duration::days(7), end - Duration::days(7)),
            Self::ThisMonth | Self::LastMonth => {
                let prev_start = start_of_previous_month(start);
                (prev_start, start)
            },
            Self::ThisYear => {
                // Previous year, same portion ([1 Jan -> same date])
                let last_year_start = start_of_year_for(now.year() - 1);
                let last_year_now = last_year_start + (now - start_of_year(now));
                (last_year_start, last_year_now)
            },
        }
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// DTO-friendly types
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub period_label: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub total_requests: u32,
    pub total_co2eq_g_p50: f64,
    pub total_energy_wh_p50: f64,
    pub total_water_l_p50: f64,
    pub vs_previous: Option<DashboardComparison>,
    pub top_models: Vec<TopModel>,
    pub daily_series: Vec<DailySeriesPoint>,
    /// Breakdown by methodology.
    ///
    /// Lists the totals for EACH methodology present in the
    /// period. Blindly summing two methodologies is scientifically
    /// wrong (they do not measure the same thing), so the separate
    /// figures go to the UI, which can show a warning if > 1.
    pub method_breakdown: Vec<MethodTotal>,
    /// `true` if the period holds entries from more than one
    /// methodology. The frontend must then show a methodology warning
    /// (see `web/src/routes/suivi/+page.svelte`).
    pub warning_multi_method: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodTotal {
    pub method: sobria_core::EmpreinteMethod,
    pub request_count: u32,
    pub total_co2eq_g_p50: f64,
    pub total_energy_wh_p50: f64,
    pub total_water_l_p50: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardComparison {
    pub previous_total_co2eq_g_p50: f64,
    pub delta_co2eq_pct: f64,
    pub previous_total_requests: u32,
    pub delta_requests_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopModel {
    pub model_id: String,
    pub request_count: u32,
    pub total_co2eq_g_p50: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DailySeriesPoint {
    /// Date in "YYYY-MM-DD" format (UTC).
    pub date: String,
    pub request_count: u32,
    pub co2eq_g_p50: f64,
    pub energy_wh_p50: f64,
    pub water_l_p50: f64,
}

// ─────────────────────────────────────────────────────────────────────────────
// Main aggregation
// ─────────────────────────────────────────────────────────────────────────────

/// Computes a dashboard summary for the given period.
///
/// `now` is usually `Utc::now()` but is a parameter so tests stay
/// deterministic. GDPR-purged entries are **counted**, but their
/// numeric values are left out of the sums (sentinel detected).
#[must_use]
pub fn aggregate(
    entries: &[AuditEntry],
    period: DashboardPeriod,
    now: DateTime<Utc>,
) -> DashboardSummary {
    let (start, end) = period.window(now);
    let (prev_start, prev_end) = period.previous_window(now);

    let current = aggregate_window(entries, start, end);
    let previous = aggregate_window(entries, prev_start, prev_end);

    // Top 5 models
    let mut top: Vec<TopModel> = current
        .by_model
        .into_iter()
        .map(|(model_id, stats)| TopModel {
            model_id,
            request_count: stats.requests,
            total_co2eq_g_p50: stats.co2,
        })
        .collect();
    top.sort_by(|a, b| {
        b.total_co2eq_g_p50
            .partial_cmp(&a.total_co2eq_g_p50)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    top.truncate(5);

    // Daily series: one point per day of the period, even if empty.
    let mut daily: Vec<DailySeriesPoint> = Vec::new();
    let mut cursor = start_of_day(start);
    while cursor < end {
        let key = cursor.format("%Y-%m-%d").to_string();
        let point = current.by_day.get(&key).cloned().unwrap_or_default();
        daily.push(DailySeriesPoint {
            date: key,
            request_count: point.requests,
            co2eq_g_p50: point.co2,
            energy_wh_p50: point.energy,
            water_l_p50: point.water,
        });
        cursor += Duration::days(1);
    }

    let vs_previous = if previous.total_requests > 0 || current.total_requests > 0 {
        let delta_co2_pct = if previous.total_co2eq > 0.0 {
            ((current.total_co2eq - previous.total_co2eq) / previous.total_co2eq) * 100.0
        } else if current.total_co2eq > 0.0 {
            100.0
        } else {
            0.0
        };
        let delta_req_pct = if previous.total_requests > 0 {
            ((f64::from(current.total_requests) - f64::from(previous.total_requests))
                / f64::from(previous.total_requests))
                * 100.0
        } else if current.total_requests > 0 {
            100.0
        } else {
            0.0
        };
        Some(DashboardComparison {
            previous_total_co2eq_g_p50: previous.total_co2eq,
            delta_co2eq_pct: delta_co2_pct,
            previous_total_requests: previous.total_requests,
            delta_requests_pct: delta_req_pct,
        })
    } else {
        None
    };

    // Breakdown by methodology in a stable order (AFNOR, then EcoLogits),
    // so the UI gets a deterministic layout.
    let mut method_breakdown: Vec<MethodTotal> = current
        .by_method
        .iter()
        .map(|(method, b)| MethodTotal {
            method: *method,
            request_count: b.requests,
            total_co2eq_g_p50: b.co2,
            total_energy_wh_p50: b.energy,
            total_water_l_p50: b.water,
        })
        .collect();
    method_breakdown.sort_by_key(|m| match m.method {
        sobria_core::EmpreinteMethod::AfnorSobria => 0,
        sobria_core::EmpreinteMethod::EcoLogits => 1,
    });
    let warning_multi_method = method_breakdown.len() > 1;

    DashboardSummary {
        period_label: period.label().to_string(),
        period_start: start,
        period_end: end,
        total_requests: current.total_requests,
        total_co2eq_g_p50: current.total_co2eq,
        total_energy_wh_p50: current.total_energy,
        total_water_l_p50: current.total_water,
        vs_previous,
        top_models: top,
        daily_series: daily,
        method_breakdown,
        warning_multi_method,
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Internals
// ─────────────────────────────────────────────────────────────────────────────

#[derive(Debug, Default, Clone)]
struct DayBucket {
    requests: u32,
    co2: f64,
    energy: f64,
    water: f64,
}

#[derive(Debug, Default, Clone)]
struct ModelBucket {
    requests: u32,
    co2: f64,
}

#[derive(Debug, Default, Clone)]
struct MethodBucket {
    requests: u32,
    co2: f64,
    energy: f64,
    water: f64,
}

#[derive(Debug, Default, Clone)]
struct WindowAggregate {
    total_requests: u32,
    total_co2eq: f64,
    total_energy: f64,
    total_water: f64,
    by_day: HashMap<String, DayBucket>,
    by_model: HashMap<String, ModelBucket>,
    by_method: HashMap<sobria_core::EmpreinteMethod, MethodBucket>,
}

fn aggregate_window(
    entries: &[AuditEntry],
    start: DateTime<Utc>,
    end: DateTime<Utc>,
) -> WindowAggregate {
    let mut agg = WindowAggregate::default();
    for entry in entries {
        if entry.timestamp < start || entry.timestamp >= end {
            continue;
        }
        agg.total_requests = agg.total_requests.saturating_add(1);
        if entry.is_purged() {
            // Counts the request, but not its numeric values.
            continue;
        }
        let parsed: Result<EstimationResult, _> =
            serde_json::from_str(&entry.estimation_result_json);
        let Ok(result) = parsed else { continue };
        let co2 = result
            .indicators
            .iter()
            .find(|i| matches!(i.indicator, Indicator::Co2Eq))
            .map_or(0.0, |i| i.interval.p50);
        let energy = result
            .indicators
            .iter()
            .find(|i| matches!(i.indicator, Indicator::Energy))
            .map_or(0.0, |i| i.interval.p50);
        let water = result
            .indicators
            .iter()
            .find(|i| matches!(i.indicator, Indicator::Water))
            .map_or(0.0, |i| i.interval.p50);

        agg.total_co2eq += co2;
        agg.total_energy += energy;
        agg.total_water += water;

        let day_key = entry.timestamp.format("%Y-%m-%d").to_string();
        let day = agg.by_day.entry(day_key).or_default();
        day.requests = day.requests.saturating_add(1);
        day.co2 += co2;
        day.energy += energy;
        day.water += water;

        let model = agg
            .by_model
            .entry(result.request.model_id.clone())
            .or_default();
        model.requests = model.requests.saturating_add(1);
        model.co2 += co2;

        // Per-methodology accumulation.
        let method_bucket = agg.by_method.entry(result.method).or_default();
        method_bucket.requests = method_bucket.requests.saturating_add(1);
        method_bucket.co2 += co2;
        method_bucket.energy += energy;
        method_bucket.water += water;
    }
    agg
}

fn start_of_day(dt: DateTime<Utc>) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(dt.year(), dt.month(), dt.day(), 0, 0, 0)
        .single()
        .unwrap_or(dt)
}

fn start_of_month(dt: DateTime<Utc>) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(dt.year(), dt.month(), 1, 0, 0, 0)
        .single()
        .unwrap_or(dt)
}

fn start_of_year(dt: DateTime<Utc>) -> DateTime<Utc> {
    start_of_year_for(dt.year())
}

fn start_of_year_for(year: i32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, 1, 1, 0, 0, 0)
        .single()
        .unwrap_or_else(|| Utc.with_ymd_and_hms(2024, 1, 1, 0, 0, 0).unwrap())
}

fn start_of_previous_month(dt: DateTime<Utc>) -> DateTime<Utc> {
    let year = dt.year();
    let month = dt.month();
    let (prev_year, prev_month) = if month == 1 {
        (year - 1, 12)
    } else {
        (year, month - 1)
    };
    let date = NaiveDate::from_ymd_opt(prev_year, prev_month, 1).expect("date valide");
    Utc.from_utc_datetime(&date.and_hms_opt(0, 0, 0).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Timelike;
    use sobria_core::{
        EmpreinteMethod, EstimationRequest, EstimationResult, Hypothesis, IndicatorValue,
        UncertaintyInterval,
    };

    fn entry_at(id: i64, ts: DateTime<Utc>, model: &str, co2: f64) -> AuditEntry {
        let result = EstimationResult {
            method: EmpreinteMethod::AfnorSobria,
            request: EstimationRequest {
                model_id: model.into(),
                tokens_in: 100,
                tokens_out_estimated: 500,
                datacenter_id: None,
                timestamp: ts,
                modalities: Vec::new(),
                overhead: sobria_core::ContextOverhead::default(),
            },
            indicators: vec![
                IndicatorValue {
                    indicator: Indicator::Co2Eq,
                    interval: UncertaintyInterval::new(co2 * 0.7, co2, co2 * 1.4).unwrap(),
                    unit: "gCO2eq".into(),
                    bins: None,
                },
                IndicatorValue {
                    indicator: Indicator::Energy,
                    interval: UncertaintyInterval::new(0.5, 1.0, 1.5).unwrap(),
                    unit: "Wh".into(),
                    bins: None,
                },
                IndicatorValue {
                    indicator: Indicator::Water,
                    interval: UncertaintyInterval::new(0.0, 0.001, 0.002).unwrap(),
                    unit: "L".into(),
                    bins: None,
                },
            ],
            equivalents: vec![],
            hypotheses: vec![Hypothesis {
                key: "x".into(),
                value: serde_json::json!(1),
                source: "test".into(),
            }],
            computed_at: ts,
            seed: 42,
        };
        AuditEntry {
            id,
            timestamp: ts,
            estimation_result_json: serde_json::to_string(&result).unwrap(),
            prev_sig: String::new(),
            sig: "x".repeat(64),
            purged_at: None,
        }
    }

    fn now_fixed() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 5, 15, 12, 0, 0).unwrap()
    }

    #[test]
    fn parse_period_handles_known_strings() {
        assert_eq!(
            DashboardPeriod::parse("today"),
            Some(DashboardPeriod::Today)
        );
        assert_eq!(
            DashboardPeriod::parse("last_7_days"),
            Some(DashboardPeriod::Last7Days)
        );
        assert_eq!(DashboardPeriod::parse("invalid"), None);
    }

    #[test]
    fn window_today_starts_at_midnight() {
        let (s, e) = DashboardPeriod::Today.window(now_fixed());
        assert_eq!(s.hour(), 0);
        assert_eq!(s.day(), 15);
        assert_eq!(e, now_fixed());
    }

    #[test]
    fn window_this_month_starts_first_day() {
        let (s, _) = DashboardPeriod::ThisMonth.window(now_fixed());
        assert_eq!(s.day(), 1);
        assert_eq!(s.month(), 5);
    }

    #[test]
    fn window_last_month_is_april() {
        let (s, e) = DashboardPeriod::LastMonth.window(now_fixed());
        assert_eq!(s.month(), 4);
        assert_eq!(s.day(), 1);
        assert_eq!(e.month(), 5);
        assert_eq!(e.day(), 1);
    }

    #[test]
    fn aggregate_empty_returns_zero_total() {
        let s = aggregate(&[], DashboardPeriod::Today, now_fixed());
        assert_eq!(s.total_requests, 0);
        assert!((s.total_co2eq_g_p50).abs() < 1e-9);
        assert!(s.vs_previous.is_none());
        assert!(s.top_models.is_empty());
    }

    #[test]
    fn aggregate_today_includes_only_today_entries() {
        // now_fixed() == 2026-05-15 12:00 UTC → the Today window is
        // [00:00, 12:00). Entries must be BEFORE 12:00.
        let today = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
        let yesterday = Utc.with_ymd_and_hms(2026, 5, 14, 10, 0, 0).unwrap();
        let entries = vec![
            entry_at(1, today, "gpt-4o-mini", 1.0),
            entry_at(2, today, "claude-3-5-sonnet", 2.0),
            entry_at(3, yesterday, "gpt-4o-mini", 5.0),
        ];
        let s = aggregate(&entries, DashboardPeriod::Today, now_fixed());
        assert_eq!(s.total_requests, 2);
        assert!((s.total_co2eq_g_p50 - 3.0).abs() < 1e-9);
    }

    #[test]
    fn aggregate_top_models_sorted_desc() {
        let today = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
        let entries = vec![
            entry_at(1, today, "gpt-4o-mini", 1.0),
            entry_at(2, today, "gpt-4o-mini", 1.5),
            entry_at(3, today, "claude-3-5-sonnet", 5.0),
        ];
        let s = aggregate(&entries, DashboardPeriod::Today, now_fixed());
        assert_eq!(s.top_models.len(), 2);
        // claude (5g) > gpt-mini (2.5g)
        assert_eq!(s.top_models[0].model_id, "claude-3-5-sonnet");
        assert_eq!(s.top_models[1].model_id, "gpt-4o-mini");
    }

    #[test]
    fn aggregate_excludes_purged_values() {
        let today = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
        let mut purged = entry_at(99, today, "gpt-4o-mini", 100.0);
        purged.estimation_result_json = sobria_audit::PURGED_SENTINEL.into();
        purged.purged_at = Some(today);
        let entries = vec![entry_at(1, today, "gpt-4o-mini", 1.0), purged];
        let s = aggregate(&entries, DashboardPeriod::Today, now_fixed());
        // 2 requests counted, only 1 value aggregated
        assert_eq!(s.total_requests, 2);
        assert!((s.total_co2eq_g_p50 - 1.0).abs() < 1e-9);
    }

    #[test]
    fn aggregate_daily_series_has_one_point_per_day() {
        let day1 = Utc.with_ymd_and_hms(2026, 5, 9, 10, 0, 0).unwrap();
        let day3 = Utc.with_ymd_and_hms(2026, 5, 11, 10, 0, 0).unwrap();
        let entries = vec![
            entry_at(1, day1, "gpt-4o-mini", 1.0),
            entry_at(2, day3, "gpt-4o-mini", 2.0),
        ];
        let s = aggregate(&entries, DashboardPeriod::Last7Days, now_fixed());
        // 7 days, so 7 points (8 if the bound is inclusive — check it)
        assert!(
            s.daily_series.len() >= 7,
            "got {} points",
            s.daily_series.len()
        );
        // day1 and day3 must have count = 1
        let day1_point = s
            .daily_series
            .iter()
            .find(|p| p.date == "2026-05-09")
            .unwrap();
        assert_eq!(day1_point.request_count, 1);
        let day3_point = s
            .daily_series
            .iter()
            .find(|p| p.date == "2026-05-11")
            .unwrap();
        assert_eq!(day3_point.request_count, 1);
        // The other days = 0
        let day2_point = s
            .daily_series
            .iter()
            .find(|p| p.date == "2026-05-10")
            .unwrap();
        assert_eq!(day2_point.request_count, 0);
    }

    #[test]
    fn aggregate_vs_previous_computes_delta_pct() {
        let today = Utc.with_ymd_and_hms(2026, 5, 15, 10, 0, 0).unwrap();
        let yesterday = Utc.with_ymd_and_hms(2026, 5, 14, 10, 0, 0).unwrap();
        let entries = vec![
            entry_at(1, today, "gpt-4o-mini", 2.0),
            entry_at(2, yesterday, "gpt-4o-mini", 1.0),
        ];
        let s = aggregate(&entries, DashboardPeriod::Today, now_fixed());
        let cmp = s.vs_previous.unwrap();
        // Yesterday 1g, today 2g → +100%
        assert!((cmp.delta_co2eq_pct - 100.0).abs() < 1e-6);
    }
}
