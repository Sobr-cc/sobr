//! The Gebru datasheet.

use chrono::{DateTime, Utc};
use serde_json::Value;
use sobria_core::EmpreinteMethod::{AfnorSobria, EcoLogits};
use sobria_export::{build_datasheet, generate_report, DatasheetOptions, ExportError};

use crate::common::{
    at, entry, entry_at, golden::assert_golden, mask, node, project, raw_entry, report_request,
    sha256_hex,
};

const DATE_MODIFIED: &str = "/@graph/1/sobria:maintenance/schema:dateModified";

fn options() -> DatasheetOptions {
    DatasheetOptions {
        estimator_version: "1.2.3".into(),
        estimator_seed: 7,
        estimator_n: 500,
        maintenance_contact: "contact@example.org".into(),
        intended_uses: "Testing.".into(),
    }
}

#[test]
fn datasheet_of_a_ledger_with_two_methods_is_pinned() {
    let entries = [
        entry_at(1, at(2026, 1, 10, 8, 0), AfnorSobria, "gpt-4o-mini", 1.0),
        entry_at(2, at(2026, 2, 20, 17, 45), EcoLogits, "mistral-large", 2.0),
        entry_at(3, at(2026, 2, 1, 12, 0), EcoLogits, "gpt-4o-mini", 3.0),
    ];
    let mut sheet = build_datasheet(&project(), &entries, &options()).jsonld;
    // The only value that comes from the clock.
    mask(&mut sheet, DATE_MODIFIED, "<time>");
    assert_golden("datasheet_two_methods", &sheet);
}

#[test]
fn datasheet_quirk_the_default_options_state_seed_42_n_10000_and_the_crate_version() {
    // The app builds its datasheets with the default options, so they state these
    // values whatever the engine really used.
    let entries = [entry(1, EcoLogits, "gpt-4o-mini", 1.0)];
    let sheet = build_datasheet(&project(), &entries, &DatasheetOptions::default());
    let datasheet = node(&sheet.jsonld, "sobria:project-1/datasheet");

    let version = env!("CARGO_PKG_VERSION");
    let process = datasheet["sobria:collectionProcess"]
        .as_str()
        .expect("text");
    assert!(
        process.contains(&format!("sobria-estimator v{version} (seed 42, N=10000)")),
        "{process}"
    );
    let maintenance = &datasheet["sobria:maintenance"];
    assert_eq!(maintenance["schema:softwareVersion"], version);
    assert_eq!(maintenance["sobria:contact"], "voir paramètres Sobr.ia");
    assert_eq!(
        datasheet["sobria:uses"],
        "Recherche scientifique, audit interne, reporting CSRD."
    );
}

fn pretty(jsonld: &Value) -> String {
    serde_json::to_string_pretty(jsonld).expect("pretty JSON")
}

#[test]
fn datasheet_quirk_date_modified_is_the_clock_and_part_of_the_hashed_content() {
    let entries = [entry(1, EcoLogits, "gpt-4o-mini", 1.0)];
    let before = Utc::now();
    let first = build_datasheet(&project(), &entries, &options());
    let after = Utc::now();

    // `schema:dateModified` is the time of the call ...
    let stamp = first
        .jsonld
        .pointer(DATE_MODIFIED)
        .and_then(Value::as_str)
        .expect("a string");
    let stamp = DateTime::parse_from_rfc3339(stamp)
        .expect("an RFC 3339 time")
        .with_timezone(&Utc);
    assert!(before <= stamp && stamp <= after, "{stamp}");

    // ... and the hash covers the pretty-printed document, so it covers that time:
    // the "SHA-256 of the canonical JSON-LD" changes on every call.
    assert_eq!(first.sha256, sha256_hex(pretty(&first.jsonld).as_bytes()));
    let mut other = first.jsonld.clone();
    mask(&mut other, DATE_MODIFIED, "1970-01-01T00:00:00+00:00");
    assert_ne!(first.sha256, sha256_hex(pretty(&other).as_bytes()));

    // It is the only value that comes from the clock.
    let mut second = build_datasheet(&project(), &entries, &options()).jsonld;
    let mut first = first.jsonld;
    mask(&mut first, DATE_MODIFIED, "<time>");
    mask(&mut second, DATE_MODIFIED, "<time>");
    assert_eq!(first, second);
}

#[test]
fn malformed_payload_quirk_the_report_fails_where_the_datasheet_skips_the_entry() {
    let entries = [
        entry(1, EcoLogits, "gpt-4o-mini", 1.0),
        raw_entry(2, at(2026, 1, 15, 10, 0), "{not json"),
    ];

    // The CSRD report refuses the whole period ...
    let error = generate_report(&report_request("fr"), &entries).expect_err("a bad payload");
    assert!(
        matches!(&error, ExportError::InvalidAuditPayload(m) if m.starts_with("entry 2:")),
        "{error:?}"
    );

    // ... and the datasheet counts the entry but reads nothing from it.
    let composition = build_datasheet(&project(), &entries, &options()).composition;
    assert_eq!(composition.total_requests, 2);
    assert_eq!(composition.unique_models, ["gpt-4o-mini"]);
    assert!((composition.total_co2eq_g_p50 - 1.0).abs() < 1e-9);
}
