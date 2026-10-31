//! Helpers of the characterization tests of `sobria-export` (SOB-221): fixed
//! ledger entries and requests, and readers for a generated PDF.
//!
//! Each test file uses a different part of this module.
#![allow(dead_code)]

#[path = "../../../../../test-support/golden.rs"]
pub mod golden;

use chrono::{DateTime, TimeZone, Utc};
use printpdf::{Op, PdfDocument, PdfPage, PdfParseOptions, PdfWarnMsg, TextItem};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sobria_audit::AuditEntry;
use sobria_core::{
    ContextOverhead, EmpreinteMethod, EstimationRequest, EstimationResult, Indicator,
    IndicatorValue, UncertaintyInterval,
};
use sobria_export::{generate_report, ProjectMeta, ReportArtifacts, ReportRequest};

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
        equivalents: Vec::new(),
        hypotheses: Vec::new(),
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

/// The report of the first quarter of 2026 for these entries.
pub fn report(locale: &str, entries: &[AuditEntry]) -> ReportArtifacts {
    generate_report(&report_request(locale), entries).expect("report")
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

/// The node of a JSON-LD `@graph` with this `@id`.
pub fn node<'a>(document: &'a Value, id: &str) -> &'a Value {
    document["@graph"]
        .as_array()
        .expect("a @graph")
        .iter()
        .find(|n| n["@id"] == id)
        .unwrap_or_else(|| panic!("no node with the @id {id}"))
}

/// Replaces the value at a JSON pointer.
pub fn mask(value: &mut Value, pointer: &str, placeholder: &str) {
    *value
        .pointer_mut(pointer)
        .unwrap_or_else(|| panic!("nothing at {pointer}")) = json!(placeholder);
}

/// SHA-256 in lower-case hexadecimal.
pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

/// Reads a PDF back with the parser of `printpdf`: the streams are compressed,
/// so the text cannot be read from the bytes.
fn parse(pdf: &[u8]) -> (PdfDocument, Vec<PdfWarnMsg>) {
    let mut warnings = Vec::new();
    let document = PdfDocument::parse(pdf, &PdfParseOptions::default(), &mut warnings)
        .expect("a PDF that printpdf can read back");
    (document, warnings)
}

/// The pages of a PDF as `printpdf` reads them: no date, no document id.
pub fn pdf_pages(pdf: &[u8]) -> Vec<PdfPage> {
    parse(pdf).0.pages
}

/// The lines of text a PDF shows, in drawing order: one per `ShowText`
/// operation. (`PdfDocument::extract_text` trims and drops lines, which would
/// hide the indentation of the report.)
pub fn pdf_text(pdf: &[u8]) -> Vec<String> {
    let (document, warnings) = parse(pdf);
    let lines: Vec<String> = document
        .pages
        .iter()
        .flat_map(|page| page.ops.iter())
        .filter_map(|op| match op {
            Op::ShowText { items } => Some(
                items
                    .iter()
                    .filter_map(|item| match item {
                        TextItem::Text(text) => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<String>(),
            ),
            _ => None,
        })
        .collect();
    assert!(
        !lines.is_empty(),
        "printpdf read no text from the PDF: {warnings:?}"
    );
    lines
}

/// The text of the report of the first quarter of 2026 for these entries.
pub fn report_text(entries: &[AuditEntry]) -> Vec<String> {
    pdf_text(&report("fr", entries).pdf_bytes)
}
