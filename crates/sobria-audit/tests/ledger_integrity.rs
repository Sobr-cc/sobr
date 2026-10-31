//! Characterization of the audit ledger signatures, chain checks, schema, export and purge
//! (SOB-223).
//!
//! Entries are fixed and inserted with raw SQL, so no test depends on the clock. The golden
//! signatures are SHA-256 of `<rfc3339 timestamp>|<payload>|<prev_sig>`, computed with Python's
//! `hashlib`, not with the code under test. Tests named `quirk_*` pin a weakness as it is today.

use std::{
    panic::{catch_unwind, AssertUnwindSafe},
    path::PathBuf,
};

use chrono::{DateTime, Utc};
use rusqlite::{params, Connection};
use sobria_audit::{AuditEntry, AuditLedger, PURGED_SENTINEL};

const T1: &str = "2026-05-12T14:32:08+00:00";
const T2: &str = "2026-05-12T14:35:41.500+00:00";
const T3: &str = "2026-05-13T09:02:17.123456789+00:00";
const T4: &str = "2026-05-14T18:47:55.000001+00:00";

struct Row {
    timestamp: &'static str,
    payload: &'static str,
    prev_sig: &'static str,
    sig: &'static str,
}

const SIG_1: &str = "b096768361820da95473ef970d37f83727d5f19e2cbe0e5c24597356436513fc";
const SIG_2: &str = "05070934732e11788f13833b6b37b67b68b5f0440389c0a1745956bc1bb7437f";
const SIG_3: &str = "4724cbc776381e43c5577a066b067590cce1352ece30695c921c020469f080b3";

const CHAIN: [Row; 3] = [
    Row {
        timestamp: T1,
        payload: r#"{"n":1}"#,
        prev_sig: "",
        sig: SIG_1,
    },
    Row {
        timestamp: T2,
        payload: r#"{"n":2}"#,
        prev_sig: SIG_1,
        sig: SIG_2,
    },
    Row {
        timestamp: T3,
        payload: r#"{"n":3}"#,
        prev_sig: SIG_2,
        sig: SIG_3,
    },
];

fn utc(timestamp: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(timestamp)
        .unwrap()
        .with_timezone(&Utc)
}

fn entry(timestamp: &str, payload: &str, prev_sig: &str) -> AuditEntry {
    AuditEntry {
        id: 1,
        timestamp: utc(timestamp),
        estimation_result_json: payload.into(),
        prev_sig: prev_sig.into(),
        sig: String::new(),
        purged_at: None,
    }
}

struct Fixture {
    _dir: tempfile::TempDir,
    path: PathBuf,
    ledger: AuditLedger,
}

impl Fixture {
    fn new(rows: &[Row]) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("audit.sqlite");
        let ledger = AuditLedger::open(&path).unwrap();
        let conn = Connection::open(&path).unwrap();
        for row in rows {
            conn.execute(
                "INSERT INTO audit_entries (timestamp, estimation_result_json, prev_sig, sig) \
                 VALUES (?1, ?2, ?3, ?4)",
                params![row.timestamp, row.payload, row.prev_sig, row.sig],
            )
            .unwrap();
        }
        Self {
            _dir: dir,
            path,
            ledger,
        }
    }

    fn sql(&self, statement: &str) {
        Connection::open(&self.path)
            .unwrap()
            .execute(statement, [])
            .unwrap();
    }

    fn exported(&self) -> (usize, String) {
        let mut out = Vec::new();
        let count = self.ledger.export_ndjson(&mut out).unwrap();
        (count, String::from_utf8(out).unwrap())
    }
}

// ─── Signature vectors ────────────────────────────────────────────────────

#[test]
fn signature_of_a_genesis_entry_is_pinned() {
    let e = entry(T1, r#"{"co2eq_p50":2.14}"#, "");

    assert_eq!(
        e.compute_sig(),
        "a1971d7cc94a858279230af3b1d54e53375e008b3334530dc2939e65912b02e3"
    );
}

#[test]
fn signatures_with_fractional_seconds_a_chain_link_and_a_purged_payload_are_pinned() {
    assert_eq!(
        entry(T2, r#"{"co2eq_p50":9.7}"#, "abc").compute_sig(),
        "06c4bc7779ae79bf780900c14e7215f3075a58d4457167b3ae5225cbf14c05a9"
    );
    assert_eq!(
        entry(T4, r#"{"model":"Modèle é"}"#, "abc").compute_sig(),
        "ce3a53cc111206fa1f250578f0fd24d6b98bfae3aa5360e3af88faf1e4207f08"
    );
    assert_eq!(
        entry(T3, PURGED_SENTINEL, "").compute_sig(),
        "7e9ed4d5cc448784559368219979ca9d4fb1862e9909c50aff4c79d2c60da3ad"
    );
}

#[test]
fn signatures_of_a_three_entry_chain_are_pinned() {
    for row in &CHAIN {
        let e = entry(row.timestamp, row.payload, row.prev_sig);

        assert_eq!(e.compute_sig(), row.sig, "{}", row.payload);
    }
}

#[test]
fn the_signature_ignores_the_id_the_stored_signature_and_the_purge_date() {
    let mut e = entry(T1, r#"{"co2eq_p50":2.14}"#, "");
    e.id = 99;
    e.sig = "anything".into();
    e.purged_at = Some(utc(T4));

    assert_eq!(
        e.compute_sig(),
        "a1971d7cc94a858279230af3b1d54e53375e008b3334530dc2939e65912b02e3"
    );
}

// SOB-237 (the separator case is noted there).
#[test]
fn quirk_values_shifted_across_the_separator_give_the_same_signature() {
    let a = entry(T1, "a|b", "c");
    let b = entry(T1, "a", "b|c");

    assert_eq!(a.compute_sig(), b.compute_sig());
    assert_eq!(
        a.compute_sig(),
        "79be2cb2fccf4a815bb524cbff024ad9e26ee8af11a3424340299f5a18fdec1f"
    );
}

// ─── What verify_chain catches and misses ─────────────────────────────────

#[test]
fn the_fixed_chain_verifies() {
    let f = Fixture::new(&CHAIN);

    let report = f.ledger.verify_chain().unwrap();

    assert!(report.valid, "{}", report.message);
    assert_eq!(report.total_entries, 3);
    assert_eq!(report.first_invalid_id, None);
    assert_eq!(report.message, "chaîne intègre : 3 entrée(s)");
}

#[test]
fn removing_the_first_entry_breaks_the_chain() {
    let f = Fixture::new(&CHAIN);
    f.sql("DELETE FROM audit_entries WHERE id = 1");

    let report = f.ledger.verify_chain().unwrap();

    assert!(!report.valid);
    assert_eq!(report.first_invalid_id, Some(2));
    assert_eq!(report.total_entries, 1);
    assert_eq!(
        report.message,
        "entrée 2 : prev_sig \"b096768361820da9\" != sig précédent \"\""
    );
}

// SOB-237
#[test]
fn quirk_cutting_the_last_entries_is_not_detected() {
    let f = Fixture::new(&CHAIN);
    f.sql("DELETE FROM audit_entries WHERE id > 1");

    let report = f.ledger.verify_chain().unwrap();
    assert!(report.valid, "{}", report.message);
    assert_eq!(report.total_entries, 1);

    f.sql("DELETE FROM audit_entries");
    let report = f.ledger.verify_chain().unwrap();
    assert!(report.valid, "{}", report.message);
    assert_eq!(report.total_entries, 0);
}

// SOB-237
#[test]
fn quirk_editing_the_method_column_is_not_detected() {
    let f = Fixture::new(&CHAIN);
    f.sql("UPDATE audit_entries SET method = 'ecologits' WHERE id = 2");

    let report = f.ledger.verify_chain().unwrap();

    assert!(report.valid, "{}", report.message);
    assert_eq!(report.total_entries, 3);
}

// SOB-237
#[test]
fn quirk_a_row_marked_purged_skips_its_own_signature_check() {
    let f = Fixture::new(&CHAIN);
    f.sql(
        "UPDATE audit_entries SET estimation_result_json = 'PURGED', sig = 'forged', \
         purged_at = '2026-06-01T00:00:00+00:00' WHERE id = 3",
    );

    let report = f.ledger.verify_chain().unwrap();

    assert!(report.valid, "{}", report.message);
    assert_eq!(report.total_entries, 3);
}

#[test]
fn a_purged_payload_without_a_purge_date_is_still_checked() {
    let f = Fixture::new(&CHAIN);
    f.sql("UPDATE audit_entries SET estimation_result_json = 'PURGED' WHERE id = 2");

    let report = f.ledger.verify_chain().unwrap();

    assert!(!report.valid);
    assert_eq!(report.first_invalid_id, Some(2));
    assert_eq!(
        report.message,
        "entrée 2 : sig stocké != sig recalculé (probable tampering)"
    );
}

// SOB-238
#[test]
fn quirk_a_prev_sig_with_non_ascii_text_makes_verify_chain_panic() {
    let f = Fixture::new(&CHAIN);
    // 16 bytes cut the third character of "aéééé…" in the middle of an "é".
    f.sql(&format!(
        "UPDATE audit_entries SET prev_sig = 'a{}' WHERE id = 2",
        "é".repeat(10)
    ));

    let outcome = catch_unwind(AssertUnwindSafe(|| f.ledger.verify_chain()));

    let panic = outcome.expect_err("verify_chain should panic");
    let message = panic.downcast_ref::<String>().expect("a formatted message");
    assert!(message.contains("is not a char boundary"), "{message}");
}

// ─── Schema ───────────────────────────────────────────────────────────────

#[test]
fn open_creates_the_v2_table_the_two_indexes_and_the_wal_mode() {
    let f = Fixture::new(&[]);
    let conn = Connection::open(&f.path).unwrap();

    let mut stmt = conn.prepare("PRAGMA table_info(audit_entries)").unwrap();
    let columns: Vec<(String, String, i64, Option<String>, i64)> = stmt
        .query_map([], |r| {
            Ok((r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?, r.get(5)?))
        })
        .unwrap()
        .map(Result::unwrap)
        .collect();
    let columns: Vec<(&str, &str, i64, Option<&str>, i64)> = columns
        .iter()
        .map(|(n, t, nn, d, pk)| (n.as_str(), t.as_str(), *nn, d.as_deref(), *pk))
        .collect();
    // `method` and its index are written by `append` and never read.
    assert_eq!(
        columns,
        [
            ("id", "INTEGER", 0, None, 1),
            ("timestamp", "TEXT", 1, None, 0),
            ("estimation_result_json", "TEXT", 1, None, 0),
            ("prev_sig", "TEXT", 1, None, 0),
            ("sig", "TEXT", 1, None, 0),
            ("purged_at", "TEXT", 0, None, 0),
            ("method", "TEXT", 1, Some("'afnor_sobria'"), 0),
        ]
    );

    let mut stmt = conn
        .prepare(
            "SELECT m.name, i.name FROM sqlite_master m, pragma_index_info(m.name) i \
             WHERE m.type = 'index' AND m.name NOT LIKE 'sqlite_%' ORDER BY m.name",
        )
        .unwrap();
    let indexes: Vec<(String, String)> = stmt
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(
        indexes,
        [
            ("idx_audit_method".to_string(), "method".to_string()),
            ("idx_audit_timestamp".to_string(), "timestamp".to_string()),
        ]
    );

    let table_sql: String = conn
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name = 'audit_entries'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(table_sql.contains("AUTOINCREMENT"), "{table_sql}");
    let mode: String = conn
        .query_row("PRAGMA journal_mode", [], |r| r.get(0))
        .unwrap();
    assert_eq!(mode, "wal");
}

// ─── export_ndjson ────────────────────────────────────────────────────────

#[test]
fn export_ndjson_writes_the_exact_lines_of_fixed_entries() {
    let f = Fixture::new(&CHAIN);
    f.sql(
        "UPDATE audit_entries SET estimation_result_json = 'PURGED', \
         purged_at = '2026-06-01T08:00:00+00:00' WHERE id = 2",
    );

    let (count, text) = f.exported();

    assert_eq!(count, 3);
    let expected = [
        format!(
            r#"{{"id":1,"timestamp":"2026-05-12T14:32:08Z","estimation_result_json":"{{\"n\":1}}","prev_sig":"","sig":"{SIG_1}","purged_at":null}}"#
        ),
        format!(
            r#"{{"id":2,"timestamp":"2026-05-12T14:35:41.500Z","estimation_result_json":"PURGED","prev_sig":"{SIG_1}","sig":"{SIG_2}","purged_at":"2026-06-01T08:00:00Z"}}"#
        ),
        format!(
            r#"{{"id":3,"timestamp":"2026-05-13T09:02:17.123456789Z","estimation_result_json":"{{\"n\":3}}","prev_sig":"{SIG_2}","sig":"{SIG_3}","purged_at":null}}"#
        ),
    ];
    assert_eq!(text, expected.join("\n") + "\n");
}

#[test]
fn export_ndjson_of_an_empty_ledger_writes_nothing() {
    let f = Fixture::new(&[]);

    let (count, text) = f.exported();

    assert_eq!(count, 0);
    assert_eq!(text, "");
}

// ─── purge_before ─────────────────────────────────────────────────────────

fn stored(f: &Fixture) -> Vec<(i64, String, String, String, Option<String>)> {
    let conn = Connection::open(&f.path).unwrap();
    let mut stmt = conn
        .prepare(
            "SELECT id, estimation_result_json, prev_sig, sig, purged_at \
             FROM audit_entries ORDER BY id",
        )
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))
        })
        .unwrap();
    rows.map(Result::unwrap).collect()
}

#[test]
fn purge_before_keeps_an_entry_exactly_at_the_cutoff() {
    let mut f = Fixture::new(&CHAIN);

    let purged = f.ledger.purge_before(utc(T2)).unwrap();

    assert_eq!(purged, 1);
    let rows = stored(&f);
    assert_eq!(rows[0].1, PURGED_SENTINEL);
    assert!(rows[0].4.is_some());
    assert_eq!(rows[1].1, CHAIN[1].payload);
    assert_eq!(rows[1].4, None);
    assert_eq!(rows[2].1, CHAIN[2].payload);
    assert!(f.ledger.verify_chain().unwrap().valid);
}

#[test]
fn purge_before_returns_the_count_and_a_second_run_purges_nothing() {
    let mut f = Fixture::new(&CHAIN);

    assert_eq!(f.ledger.purge_before(utc(T4)).unwrap(), 3);
    assert_eq!(f.ledger.purge_before(utc(T4)).unwrap(), 0);
    assert_eq!(f.ledger.purge_before(utc(T1)).unwrap(), 0);
}

#[test]
fn purged_rows_keep_their_signature_and_chain_link() {
    let mut f = Fixture::new(&CHAIN);

    f.ledger.purge_before(utc(T4)).unwrap();

    let rows = stored(&f);
    let links: Vec<(&str, &str)> = rows.iter().map(|r| (r.2.as_str(), r.3.as_str())).collect();
    assert_eq!(links, [("", SIG_1), (SIG_1, SIG_2), (SIG_2, SIG_3)]);
    assert!(rows.iter().all(|r| r.1 == PURGED_SENTINEL && r.4.is_some()));
    let report = f.ledger.verify_chain().unwrap();
    assert!(report.valid, "{}", report.message);
}
