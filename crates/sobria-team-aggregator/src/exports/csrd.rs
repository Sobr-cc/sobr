//! CSRD/AFNOR SPEC 2314 PDF export — reuses `sobria-export::generate_report`.
//!
//! Approach: each `team.estimations` estimation becomes an `AuditEntry`
//! shim carrying a minimal `EstimationResult`. The aggregation chain + the
//! PDF (printpdf) of the `sobria-export` crate do the rest — so the team
//! PDF has the same shape as the desktop one (CSRD visual consistency).
//!
//! If the estimation has no P5/P95 (the v0.6.0 extension payload is a
//! point estimate), they are synthesized as `P50 × 0.85` / `P50 × 1.15`
//! (30 % width as the default convention). Documented in the PDF through
//! the AFNOR methodology; statistical consistency is preserved
//! (sum of P50 = same value).

use chrono::{DateTime, Utc};
use sobria_audit::AuditEntry;
use sobria_core::{
    EmpreinteMethod, EstimationRequest, EstimationResult, Indicator, IndicatorValue,
    UncertaintyInterval,
};
use sobria_export::{generate_report, ReportArtifacts, ReportRequest};

use crate::error::AggregatorError;
use crate::storage::estimations::EstimationRow;

/// Default uncertainty width when the payload has no P5/P95.
const DEFAULT_UNCERTAINTY_PCT: f64 = 0.15;

/// Builds the PDF + PROV-O sidecar of the aggregated team report.
pub fn build_report(
    rows: &[EstimationRow],
    organization_name: &str,
    period_start: DateTime<Utc>,
    period_end: DateTime<Utc>,
) -> Result<ReportArtifacts, AggregatorError> {
    let entries: Vec<AuditEntry> = rows
        .iter()
        .enumerate()
        .map(|(i, row)| audit_entry_from_row(i as i64 + 1, row))
        .collect();

    let req = ReportRequest {
        period_start,
        period_end,
        organization_name: organization_name.to_string(),
        locale: "fr".to_string(),
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        // The extension/team-aggregator is a point estimate (no Monte-Carlo) —
        // keep a canonical seed for PROV-O traceability.
        estimator_seed: 42,
        estimator_n: 1,
    };

    generate_report(&req, &entries).map_err(map_export_err)
}

fn audit_entry_from_row(audit_id: i64, row: &EstimationRow) -> AuditEntry {
    let result = estimation_result_from_row(row);
    let payload = serde_json::to_string(&result).expect("EstimationResult serialise");
    AuditEntry {
        id: audit_id,
        timestamp: row.ts,
        estimation_result_json: payload,
        prev_sig: String::new(),
        sig: format!("team-shim-{}", row.id),
        purged_at: None,
    }
}

fn estimation_result_from_row(row: &EstimationRow) -> EstimationResult {
    let method = EmpreinteMethod::from_str(&row.method).unwrap_or(EmpreinteMethod::AfnorSobria);

    let p50 = row.gco2eq_p50.max(0.0);
    let (p5, p95) = match (row.gco2eq_p5, row.gco2eq_p95) {
        (Some(lo), Some(hi)) => (lo.max(0.0).min(p50), hi.max(p50)),
        _ => (
            p50 * (1.0 - DEFAULT_UNCERTAINTY_PCT),
            p50 * (1.0 + DEFAULT_UNCERTAINTY_PCT),
        ),
    };
    let co2_interval = UncertaintyInterval::new(p5, p50, p95)
        .unwrap_or_else(|_| UncertaintyInterval::point(p50).expect("p50 fini"));

    let energy_wh = row.energy_wh.max(0.0);
    let energy_interval = UncertaintyInterval::point(energy_wh).expect("energy fini ≥ 0");

    // SQLite stores water in millilitres; sobria-export expects litres.
    let water_l = (row.water_ml.max(0.0)) / 1000.0;
    let water_interval = UncertaintyInterval::point(water_l).expect("water fini ≥ 0");

    EstimationResult {
        method,
        request: EstimationRequest {
            model_id: row.model_id.clone(),
            tokens_in: row.tokens_in,
            tokens_out_estimated: row.tokens_out,
            datacenter_id: row.region.clone(),
            timestamp: row.ts,
            modalities: Vec::new(),
            overhead: sobria_core::ContextOverhead::default(),
        },
        indicators: vec![
            IndicatorValue {
                indicator: Indicator::Co2Eq,
                interval: co2_interval,
                unit: "gCO2eq".to_string(),
                bins: None,
            },
            IndicatorValue {
                indicator: Indicator::Energy,
                interval: energy_interval,
                unit: "Wh".to_string(),
                bins: None,
            },
            IndicatorValue {
                indicator: Indicator::Water,
                interval: water_interval,
                unit: "L".to_string(),
                bins: None,
            },
        ],
        equivalents: vec![],
        hypotheses: vec![],
        computed_at: row.ts,
        seed: 42,
    }
}

fn map_export_err(e: sobria_export::ExportError) -> AggregatorError {
    AggregatorError::Config(format!("sobria-export: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    fn sample_row(id: &str, ts: DateTime<Utc>, gco2eq: f64) -> EstimationRow {
        EstimationRow {
            id: id.into(),
            user_id: "u-1".into(),
            user_fingerprint: "chrome-mac-abc".into(),
            user_display_name: Some("Alice".into()),
            ts,
            method: "afnor_sobria".into(),
            model_id: "llama-3-1-70b".into(),
            tokens_in: 120,
            tokens_out: 800,
            gco2eq_p50: gco2eq,
            gco2eq_p5: None,
            gco2eq_p95: None,
            water_ml: 1500.0,
            energy_wh: 0.42,
            region: Some("FR".into()),
        }
    }

    #[test]
    fn build_report_produces_pdf_with_correct_aggregates() {
        let from = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 5, 31, 23, 59, 59).unwrap();
        let rows = vec![
            sample_row(
                "e-1",
                Utc.with_ymd_and_hms(2026, 5, 5, 10, 0, 0).unwrap(),
                0.40,
            ),
            sample_row(
                "e-2",
                Utc.with_ymd_and_hms(2026, 5, 6, 14, 0, 0).unwrap(),
                0.60,
            ),
        ];
        let r = build_report(&rows, "Acme Corp", from, to).unwrap();
        assert!(!r.pdf_bytes.is_empty());
        assert!(r.pdf_bytes.starts_with(b"%PDF-"), "magic header PDF");
        assert_eq!(r.audit_entries_count, 2);
        // P50 total = 0.4 + 0.6 = 1.0.
        assert!((r.summary.total_co2eq_g_p50 - 1.0).abs() < 1e-9);
        // The PROV-O sidecar references the PDF by its sha256.
        assert!(!r.pdf_sha256.is_empty());
        assert_eq!(r.pdf_sha256.len(), 64);
    }

    #[test]
    fn build_report_synthesizes_p5_p95_when_missing() {
        let from = Utc.with_ymd_and_hms(2026, 5, 1, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 5, 31, 23, 59, 59).unwrap();
        let rows = vec![sample_row(
            "e-1",
            Utc.with_ymd_and_hms(2026, 5, 5, 10, 0, 0).unwrap(),
            1.0,
        )];
        let r = build_report(&rows, "Test", from, to).unwrap();
        assert!((r.summary.total_co2eq_g_p5 - 0.85).abs() < 1e-9);
        assert!((r.summary.total_co2eq_g_p95 - 1.15).abs() < 1e-9);
    }

    #[test]
    fn build_report_empty_period_returns_err() {
        let from = Utc.with_ymd_and_hms(2026, 6, 1, 0, 0, 0).unwrap();
        let to = Utc.with_ymd_and_hms(2026, 6, 30, 0, 0, 0).unwrap();
        let err = build_report(&[], "Test", from, to).unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("sobria-export"), "msg = {msg}");
    }
}
