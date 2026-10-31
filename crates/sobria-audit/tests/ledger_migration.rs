//! Characterization of the audit ledger v1 -> v2 migration (SOB-160, SOB-179).
//!
//! `fixtures/ledger-v1.sqlite` is a ledger as written before the `method` column existed;
//! `fixtures/make_ledger_v1.py` rebuilds it. Tests work on a copy: opening migrates the file.

use std::path::{Path, PathBuf};

use chrono::Utc;
use rusqlite::{Connection, Row};
use sobria_audit::{AuditEntry, AuditLedger};
use sobria_core::{
    ContextOverhead, EmpreinteMethod, EstimationRequest, EstimationResult, Hypothesis, Indicator,
    IndicatorValue, UncertaintyInterval,
};

const ENTRIES: usize = 4;
const V1_COLUMNS: [&str; 6] = [
    "id",
    "timestamp",
    "estimation_result_json",
    "prev_sig",
    "sig",
    "purged_at",
];

type Entry = (i64, String, String, String, String, Option<String>);
type SchemaObject = (String, String, Option<String>);

fn legacy_copy() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ledger.sqlite");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ledger-v1.sqlite");
    std::fs::copy(fixture, &path).unwrap();
    (dir, path)
}

fn query<T>(path: &Path, sql: &str, map: impl FnMut(&Row<'_>) -> rusqlite::Result<T>) -> Vec<T> {
    let conn = Connection::open(path).unwrap();
    let mut stmt = conn.prepare(sql).unwrap();
    let rows = stmt.query_map([], map).unwrap();
    rows.map(Result::unwrap).collect()
}

fn columns(path: &Path) -> Vec<String> {
    query(path, "PRAGMA table_info(audit_entries)", |r| r.get(1))
}

fn entries(path: &Path) -> Vec<Entry> {
    query(
        path,
        "SELECT id, timestamp, estimation_result_json, prev_sig, sig, purged_at \
         FROM audit_entries ORDER BY id",
        |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        },
    )
}

fn methods(path: &Path) -> Vec<String> {
    query(path, "SELECT method FROM audit_entries ORDER BY id", |r| {
        r.get(0)
    })
}

fn schema(path: &Path) -> Vec<SchemaObject> {
    query(
        path,
        "SELECT type, name, sql FROM sqlite_master ORDER BY type, name",
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )
}

fn exported(ledger: &AuditLedger) -> Vec<AuditEntry> {
    let mut out = Vec::new();
    ledger.export_ndjson(&mut out).unwrap();
    String::from_utf8(out)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn assert_chain_intact(ledger: &AuditLedger) {
    assert_eq!(ledger.len().unwrap(), ENTRIES);
    let report = ledger.verify_chain().unwrap();
    assert!(report.valid, "{}", report.message);
    assert_eq!(report.total_entries, ENTRIES);
    assert_eq!(report.first_invalid_id, None);
}

#[test]
fn opening_a_v1_ledger_migrates_it_to_v2_and_keeps_every_entry() {
    let (_dir, path) = legacy_copy();
    assert_eq!(columns(&path), V1_COLUMNS);
    let before = entries(&path);
    assert_eq!(before.len(), ENTRIES);

    let ledger = AuditLedger::open(&path).unwrap();

    let mut v2_columns = V1_COLUMNS.to_vec();
    v2_columns.push("method");
    assert_eq!(columns(&path), v2_columns);
    let method_column = query(
        &path,
        "SELECT type, \"notnull\", dflt_value FROM pragma_table_info('audit_entries') \
         WHERE name = 'method'",
        |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, i64>(1)?,
                r.get::<_, String>(2)?,
            ))
        },
    );
    assert_eq!(
        method_column,
        [("TEXT".to_string(), 1, "'afnor_sobria'".to_string())]
    );
    assert_eq!(entries(&path), before);
    assert_eq!(methods(&path), ["afnor_sobria"; ENTRIES]);
    let indexes: Vec<String> = query(
        &path,
        "SELECT name FROM sqlite_master WHERE type = 'index' ORDER BY name",
        |r| r.get(0),
    );
    assert_eq!(indexes, ["idx_audit_method", "idx_audit_timestamp"]);
    let method_index: Vec<String> = query(
        &path,
        "SELECT name FROM pragma_index_info('idx_audit_method')",
        |r| r.get(0),
    );
    assert_eq!(method_index, ["method"]);

    assert_chain_intact(&ledger);
    let exported = exported(&ledger);
    let ids: Vec<i64> = exported.iter().map(|e| e.id).collect();
    assert_eq!(ids, [1, 2, 3, 4]);
    let purged: Vec<bool> = exported.iter().map(AuditEntry::is_purged).collect();
    assert_eq!(purged, [false, false, true, false]);
    assert_eq!(exported[0].prev_sig, "");
    for pair in exported.windows(2) {
        assert_eq!(pair[1].prev_sig, pair[0].sig);
    }
}

#[test]
fn reopening_a_migrated_ledger_changes_nothing() {
    let (_dir, path) = legacy_copy();
    drop(AuditLedger::open(&path).unwrap());
    let migrated_entries = entries(&path);
    let migrated_schema = schema(&path);

    let ledger = AuditLedger::open(&path).unwrap();

    assert_eq!(schema(&path), migrated_schema);
    assert_eq!(entries(&path), migrated_entries);
    assert_eq!(methods(&path), ["afnor_sobria"; ENTRIES]);
    assert_chain_intact(&ledger);
}

fn ecologits_result() -> EstimationResult {
    EstimationResult {
        method: EmpreinteMethod::EcoLogits,
        request: EstimationRequest {
            model_id: "gpt-4o-mini".into(),
            tokens_in: 100,
            tokens_out_estimated: 500,
            datacenter_id: None,
            timestamp: Utc::now(),
            modalities: Vec::new(),
            overhead: ContextOverhead::default(),
        },
        indicators: vec![IndicatorValue {
            indicator: Indicator::Co2Eq,
            interval: UncertaintyInterval::new(1.0, 2.0, 3.0).unwrap(),
            unit: "gCO2eq".into(),
            bins: None,
        }],
        equivalents: vec![],
        hypotheses: vec![Hypothesis {
            key: "x".into(),
            value: serde_json::json!(1.0),
            source: "test".into(),
        }],
        computed_at: Utc::now(),
        seed: 42,
    }
}

#[test]
fn appending_to_a_migrated_ledger_continues_the_ids_and_the_chain() {
    let (_dir, path) = legacy_copy();
    let before = entries(&path);
    let mut ledger = AuditLedger::open(&path).unwrap();
    let result = ecologits_result();

    let appended = ledger.append(&result).unwrap();

    assert_eq!(appended.id, 5);
    assert_eq!(appended.prev_sig, before[3].4);
    assert_eq!(appended.sig, appended.compute_sig());
    assert_eq!(
        appended.estimation_result_json,
        serde_json::to_string(&result).unwrap()
    );
    assert_eq!(appended.purged_at, None);
    let mut expected_methods = vec!["afnor_sobria"; ENTRIES];
    expected_methods.push("ecologits");
    assert_eq!(methods(&path), expected_methods);
    let after = entries(&path);
    assert_eq!(after[..ENTRIES], before[..]);
    assert_eq!(after.len(), ENTRIES + 1);
    assert_eq!(after[ENTRIES].0, 5);
    assert_eq!(after[ENTRIES].3, before[3].4);
    assert_eq!(after[ENTRIES].4, appended.sig);
    assert_eq!(ledger.len().unwrap(), ENTRIES + 1);
    let report = ledger.verify_chain().unwrap();
    assert!(report.valid, "{}", report.message);
    assert_eq!(report.total_entries, ENTRIES + 1);
    assert_eq!(report.first_invalid_id, None);
    let ids: Vec<i64> = exported(&ledger).iter().map(|e| e.id).collect();
    assert_eq!(ids, [1, 2, 3, 4, 5]);
}
