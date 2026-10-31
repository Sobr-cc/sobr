//! Characterization tests of the Gebru datasheet (SOB-221).
//!
//! They record what the datasheet says today. Tests named `quirk` pin behavior
//! the codebase audit flagged; SOB-23 (cleanup) decides what to do with it.
//!
//! To rewrite the golden files, see `test-support/golden.rs`
//! (`SOBRIA_UPDATE_GOLDEN=1`).

mod common;

use std::time::Duration;

use common::{
    at, entry, entry_at, golden::assert_golden, node, project, raw_entry, report_request,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sobria_core::EmpreinteMethod::{AfnorSobria, EcoLogits};
use sobria_export::{build_datasheet, generate_report, DatasheetOptions, ExportError};

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
    *sheet.pointer_mut(DATE_MODIFIED).expect("dateModified") = json!("<time>");
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

/// `jsonld` without the one value that comes from the clock.
fn without_date_modified(mut jsonld: Value) -> Value {
    *jsonld.pointer_mut(DATE_MODIFIED).expect("dateModified") = Value::Null;
    jsonld
}

#[test]
fn datasheet_quirk_date_modified_is_the_clock_and_part_of_the_hashed_content() {
    let entries = [entry(1, EcoLogits, "gpt-4o-mini", 1.0)];
    let first = build_datasheet(&project(), &entries, &options());
    std::thread::sleep(Duration::from_millis(5));
    let second = build_datasheet(&project(), &entries, &options());

    // Same input, other hash: the "SHA-256 of the canonical JSON-LD" changes on
    // every call because of `schema:dateModified`.
    assert_ne!(
        first.jsonld.pointer(DATE_MODIFIED),
        second.jsonld.pointer(DATE_MODIFIED)
    );
    assert_ne!(first.sha256, second.sha256);
    assert_eq!(
        without_date_modified(first.jsonld.clone()),
        without_date_modified(second.jsonld)
    );
    // The hash covers the pretty-printed document, `dateModified` included.
    let pretty = serde_json::to_string_pretty(&first.jsonld).expect("pretty JSON");
    assert_eq!(first.sha256, format!("{:x}", Sha256::digest(pretty)));
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
