//! Helpers of the characterization tests of `sobria-export` (SOB-221): fixed
//! ledger entries and requests, and a reader for the text of a generated PDF.
//!
//! Each test file uses a different part of this module.
#![allow(dead_code)]

#[path = "../../../../test-support/golden.rs"]
pub mod golden;

use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use sobria_audit::AuditEntry;
use sobria_core::{
    ContextOverhead, EmpreinteMethod, EstimationRequest, EstimationResult, Hypothesis, Indicator,
    IndicatorValue, UncertaintyInterval,
};
use sobria_export::{ProjectMeta, ReportRequest};

pub const AFNOR_HEADER: &str = "Rapport conforme AFNOR SPEC 2314";

pub fn at(year: i32, month: u32, day: u32, hour: u32, minute: u32) -> DateTime<Utc> {
    Utc.with_ymd_and_hms(year, month, day, hour, minute, 0)
        .single()
        .expect("a valid date")
}

/// An audit entry holding a real `EstimationResult`, with no clock involved.
pub fn entry_at(
    id: i64,
    timestamp: DateTime<Utc>,
    method: EmpreinteMethod,
    model: &str,
    co2eq_p50: f64,
) -> AuditEntry {
    let interval = |p5, p50, p95| UncertaintyInterval::new(p5, p50, p95).expect("an interval");
    let result = EstimationResult {
        method,
        request: EstimationRequest {
            model_id: model.into(),
            tokens_in: 100,
            tokens_out_estimated: 500,
            datacenter_id: None,
            timestamp,
            modalities: Vec::new(),
            overhead: ContextOverhead::default(),
        },
        indicators: vec![
            IndicatorValue {
                indicator: Indicator::Co2Eq,
                interval: interval(co2eq_p50 * 0.7, co2eq_p50, co2eq_p50 * 1.4),
                unit: "gCO2eq".into(),
                bins: None,
            },
            IndicatorValue {
                indicator: Indicator::Energy,
                interval: interval(0.5, 1.0, 1.5),
                unit: "Wh".into(),
                bins: None,
            },
            IndicatorValue {
                indicator: Indicator::Water,
                interval: interval(0.0, 0.001, 0.002),
                unit: "L".into(),
                bins: None,
            },
        ],
        equivalents: vec![],
        hypotheses: vec![Hypothesis {
            key: "k".into(),
            value: serde_json::json!(1),
            source: "test".into(),
        }],
        computed_at: timestamp,
        seed: 42,
    };
    raw_entry(
        id,
        timestamp,
        &serde_json::to_string(&result).expect("serialize"),
    )
}

/// [`entry_at`] on 15 January 2026, inside [`report_request`]'s period.
pub fn entry(id: i64, method: EmpreinteMethod, model: &str, co2eq_p50: f64) -> AuditEntry {
    entry_at(id, at(2026, 1, 15, 10, 0), method, model, co2eq_p50)
}

/// An audit entry with any payload text.
pub fn raw_entry(id: i64, timestamp: DateTime<Utc>, payload: &str) -> AuditEntry {
    AuditEntry {
        id,
        timestamp,
        estimation_result_json: payload.into(),
        prev_sig: String::new(),
        sig: "x".repeat(64),
        purged_at: None,
    }
}

/// The first quarter of 2026.
pub fn report_request(locale: &str) -> ReportRequest {
    ReportRequest {
        period_start: at(2026, 1, 1, 0, 0),
        period_end: at(2026, 4, 1, 0, 0),
        organization_name: "Acme Corp".into(),
        locale: locale.into(),
        app_version: "0.2.0".into(),
        estimator_seed: 42,
        estimator_n: 10_000,
    }
}

pub fn project() -> ProjectMeta {
    ProjectMeta {
        id: 1,
        name: "Project X".into(),
        description: "A test project.".into(),
        period_start: at(2026, 1, 1, 0, 0),
        period_end: at(2026, 4, 1, 0, 0),
        tags: vec!["ai".into(), "audit".into()],
        created_at: at(2026, 1, 2, 9, 30),
    }
}

/// The node of a JSON-LD `@graph` whose `@id` starts with `prefix`.
pub fn node<'a>(document: &'a Value, prefix: &str) -> &'a Value {
    document["@graph"]
        .as_array()
        .expect("a @graph")
        .iter()
        .find(|n| n["@id"].as_str().is_some_and(|id| id.starts_with(prefix)))
        .unwrap_or_else(|| panic!("no node with an @id starting with {prefix}"))
}

/// The lines of text a PDF shows, in drawing order.
///
/// `printpdf` writes each line as a hexadecimal string followed by `Tj`, in
/// the Windows-1252 encoding, with the content stream uncompressed.
pub fn pdf_text(pdf: &[u8]) -> Vec<String> {
    let mut lines = Vec::new();
    let mut i = 0;
    while i < pdf.len() {
        if pdf[i] != b'<' || !pdf.get(i + 1).is_some_and(u8::is_ascii_hexdigit) {
            i += 1;
            continue;
        }
        let start = i + 1;
        let end = start
            + pdf[start..]
                .iter()
                .take_while(|b| b.is_ascii_hexdigit())
                .count();
        if pdf[end..].starts_with(b"> Tj") && (end - start) % 2 == 0 {
            let hex = std::str::from_utf8(&pdf[start..end]).expect("ASCII hex digits");
            let bytes = (0..hex.len())
                .step_by(2)
                .map(|k| u8::from_str_radix(&hex[k..k + 2], 16).expect("a hex byte"));
            lines.push(bytes.map(windows_1252).collect());
        }
        i = end;
    }
    lines
}

fn windows_1252(byte: u8) -> char {
    match byte {
        0x80 => '€',
        0x85 => '…',
        0x91 => '‘',
        0x92 => '’',
        0x93 => '“',
        0x94 => '”',
        0x95 => '•',
        0x96 => '–',
        0x97 => '—',
        _ => char::from(byte),
    }
}

/// The PDF without its trailer `/ID`, the only part that differs between two
/// runs on the same input (the document id is random).
pub fn without_document_id(pdf: &[u8]) -> Vec<u8> {
    let start = pdf
        .windows(4)
        .position(|w| w == b"/ID[")
        .expect("a trailer /ID");
    let end = start
        + pdf[start..]
            .iter()
            .position(|&b| b == b']')
            .expect("the end of /ID")
        + 1;
    [&pdf[..start], &pdf[end..]].concat()
}
