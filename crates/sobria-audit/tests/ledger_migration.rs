//! Characterization of the audit ledger v1 -> v2 migration (SOB-160).
//!
//! `fixtures/ledger-v1.sqlite` is a ledger as written before the `method` column existed;
//! `fixtures/make_ledger_v1.py` rebuilds it. Tests work on a copy: opening migrates the file.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use sobria_audit::{AuditEntry, AuditLedger, PURGED_SENTINEL};

const V1_COLUMNS: [&str; 6] = [
    "id",
    "timestamp",
    "estimation_result_json",
    "prev_sig",
    "sig",
    "purged_at",
];

type Row = (i64, String, String, String, String, Option<String>);
type SchemaObject = (String, String, Option<String>);

fn legacy_copy() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("ledger.sqlite");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/ledger-v1.sqlite");
    std::fs::copy(fixture, &path).unwrap();
    (dir, path)
}

fn columns(path: &Path) -> Vec<String> {
    let conn = Connection::open(path).unwrap();
    let mut stmt = conn.prepare("PRAGMA table_info(audit_entries)").unwrap();
    let names = stmt.query_map([], |r| r.get(1)).unwrap();
    names.map(Result::unwrap).collect()
}

fn rows(path: &Path) -> Vec<Row> {
    let conn = Connection::open(path).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, timestamp, estimation_result_json, prev_sig, sig, purged_at \
             FROM audit_entries ORDER BY id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get(0)?,
                r.get(1)?,
                r.get(2)?,
                r.get(3)?,
                r.get(4)?,
                r.get(5)?,
            ))
        })
        .unwrap();
    rows.map(Result::unwrap).collect()
}

fn methods(path: &Path) -> Vec<String> {
    let conn = Connection::open(path).unwrap();
    let mut stmt = conn
        .prepare("SELECT method FROM audit_entries ORDER BY id")
        .unwrap();
    let methods = stmt.query_map([], |r| r.get(0)).unwrap();
    methods.map(Result::unwrap).collect()
}

fn schema(path: &Path) -> Vec<SchemaObject> {
    let conn = Connection::open(path).unwrap();
    let mut stmt = conn
        .prepare("SELECT type, name, sql FROM sqlite_master ORDER BY type, name")
        .unwrap();
    let objects = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .unwrap();
    objects.map(Result::unwrap).collect()
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

#[test]
fn opening_a_v1_ledger_migrates_it_to_v2_and_keeps_every_entry() {
    let (_dir, path) = legacy_copy();
    assert_eq!(columns(&path), V1_COLUMNS);
    let before = rows(&path);
    assert_eq!(before.len(), 4);

    let ledger = AuditLedger::open(&path).unwrap();

    let mut v2_columns: Vec<&str> = V1_COLUMNS.to_vec();
    v2_columns.push("method");
    assert_eq!(columns(&path), v2_columns);
    assert_eq!(rows(&path), before);
    assert_eq!(methods(&path), ["afnor_sobria"; 4]);
    let indexes: Vec<String> = schema(&path)
        .into_iter()
        .filter(|(kind, _, _)| kind == "index")
        .map(|(_, name, _)| name)
        .collect();
    assert!(indexes.contains(&"idx_audit_timestamp".to_string()));
    assert!(indexes.contains(&"idx_audit_method".to_string()));

    assert_eq!(ledger.len().unwrap(), 4);
    let report = ledger.verify_chain().unwrap();
    assert!(report.valid, "{}", report.message);
    assert_eq!(report.total_entries, 4);
    assert_eq!(report.first_invalid_id, None);

    let entries = exported(&ledger);
    let ids: Vec<i64> = entries.iter().map(|e| e.id).collect();
    assert_eq!(ids, [1, 2, 3, 4]);
    let purged: Vec<bool> = entries.iter().map(AuditEntry::is_purged).collect();
    assert_eq!(purged, [false, false, true, false]);
    assert_eq!(entries[2].estimation_result_json, PURGED_SENTINEL);
    assert_eq!(entries[0].prev_sig, "");
    for pair in entries.windows(2) {
        assert_eq!(pair[1].prev_sig, pair[0].sig);
    }
}

#[test]
fn reopening_a_migrated_ledger_changes_nothing() {
    let (_dir, path) = legacy_copy();
    drop(AuditLedger::open(&path).unwrap());
    let migrated_rows = rows(&path);
    let migrated_schema = schema(&path);
    let migrated_columns = columns(&path);

    let ledger = AuditLedger::open(&path).unwrap();

    assert_eq!(columns(&path), migrated_columns);
    assert_eq!(schema(&path), migrated_schema);
    assert_eq!(rows(&path), migrated_rows);
    assert_eq!(methods(&path), ["afnor_sobria"; 4]);
    assert_eq!(ledger.len().unwrap(), 4);
    let report = ledger.verify_chain().unwrap();
    assert!(report.valid, "{}", report.message);
    assert_eq!(report.total_entries, 4);
}
