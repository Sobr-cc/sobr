//! Characterization tests of `sobria-referentiel` (SOB-220).
//!
//! They record what the crate does today with the Gold file: where `load`
//! looks when the variable is not set, what `snapshot_date` is, what a broken
//! file gives, and what the hash covers. Tests named `quirk` pin behavior the
//! codebase audit flagged; SOB-20 (cleanup) decides what to do with it.
//!
//! The default path depends on the current folder and on the environment of the
//! process. A test must not change either: the cases that need them run this
//! same test binary again as a child process, with its own folder and
//! environment (see [`run_child`]).

use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, SystemTime},
};

use chrono::{DateTime, Utc};
use rusqlite::Connection;
use sobria_referentiel::{
    load, Referentiel, ReferentielError, ReferentielStatus, DEFAULT_REFERENTIEL_PATH,
    REFERENTIEL_PATH_ENV,
};

/// A Gold file with `sources` and `model_overview`.
fn gold(dir: &Path, sources: usize, models: usize) -> PathBuf {
    let path = dir.join("referentiel.sqlite");
    let conn = Connection::open(&path).expect("create the database");
    conn.execute_batch(
        "CREATE TABLE sources (id TEXT PRIMARY KEY, name TEXT NOT NULL);
         CREATE TABLE model_overview (id TEXT PRIMARY KEY, name TEXT);",
    )
    .expect("create the tables");
    for i in 0..sources {
        conn.execute(
            "INSERT INTO sources(id, name) VALUES (?1, ?2)",
            rusqlite::params![format!("src_{i}"), format!("Source {i}")],
        )
        .expect("insert a source");
    }
    for i in 0..models {
        conn.execute(
            "INSERT INTO model_overview(id, name) VALUES (?1, ?2)",
            rusqlite::params![format!("m_{i}"), format!("model {i}")],
        )
        .expect("insert a model");
    }
    path
}

fn status_of(path: &Path) -> ReferentielStatus {
    Referentiel::open(path)
        .expect("open")
        .status()
        .expect("status")
}

// ─── Default path ───────────────────────────────────────────────────────────

/// Tells a test run as a child process which scenario to play.
const CHILD: &str = "SOBRIA_REFERENTIEL_TEST_CHILD";

/// Runs the test `name` of this binary again in `folder`, without
/// `SOBRIA_REFERENTIEL_PATH`, and returns its `CHILD …` lines.
fn run_child(name: &str, scenario: &str, folder: &Path) -> Vec<String> {
    let output = Command::new(std::env::current_exe().expect("this test binary"))
        .args(["--exact", name, "--nocapture", "--test-threads=1"])
        .env(CHILD, scenario)
        .env_remove(REFERENTIEL_PATH_ENV)
        .current_dir(folder)
        .output()
        .expect("run the child test");
    assert!(
        output.status.success(),
        "the child failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    // The harness writes "test <name> ... " before the first line of the test.
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|l| l.split_once("CHILD ").map(|(_, rest)| rest.to_owned()))
        .collect()
}

#[test]
fn load_quirk_without_the_variable_the_default_path_is_relative_to_the_current_folder() {
    if std::env::var(CHILD).is_ok_and(|s| s == "found") {
        let referentiel = load().expect("the default file exists in the current folder");
        let status = referentiel.status().expect("status");
        println!("CHILD path={}", referentiel.path().display());
        println!("CHILD status_path={}", status.path);
        println!("CHILD sources={}", status.source_count);
        return;
    }

    // `data/gold/referentiel.sqlite` under a folder that is not the folder of
    // the test.
    let tmp = tempfile::tempdir().expect("tmp");
    let gold_dir = tmp.path().join("data").join("gold");
    fs::create_dir_all(&gold_dir).expect("create data/gold");
    fs::rename(gold(tmp.path(), 3, 1), gold_dir.join("referentiel.sqlite")).expect("move");

    let lines = run_child(
        "load_quirk_without_the_variable_the_default_path_is_relative_to_the_current_folder",
        "found",
        tmp.path(),
    );
    assert!(
        lines.contains(&format!("path={DEFAULT_REFERENTIEL_PATH}")),
        "{lines:?}"
    );
    // The documentation of `ReferentielStatus::path` says "chemin absolu": it is
    // the path as given, here the relative default.
    assert!(
        lines.contains(&format!("status_path={DEFAULT_REFERENTIEL_PATH}")),
        "{lines:?}"
    );
    assert!(lines.contains(&"sources=3".to_string()), "{lines:?}");
}

#[test]
fn load_without_the_variable_does_not_find_the_default_path_from_another_folder() {
    if std::env::var(CHILD).is_ok_and(|s| s == "missing") {
        let error = load().err().expect("nothing at the default path here");
        println!("CHILD error={error}");
        println!(
            "CHILD not_found={}",
            matches!(error, ReferentielError::NotFound(_))
        );
        return;
    }

    let empty = tempfile::tempdir().expect("tmp");
    let lines = run_child(
        "load_without_the_variable_does_not_find_the_default_path_from_another_folder",
        "missing",
        empty.path(),
    );
    assert!(lines.contains(&"not_found=true".to_string()), "{lines:?}");
    let message = lines
        .iter()
        .find_map(|l| l.strip_prefix("error="))
        .expect("an error line");
    assert!(
        message.contains(&format!("introuvable : {DEFAULT_REFERENTIEL_PATH}")),
        "{message}"
    );
}

// ─── snapshot_date ──────────────────────────────────────────────────────────

fn set_modified(path: &Path, time: SystemTime) {
    fs::File::options()
        .write(true)
        .open(path)
        .expect("open for writing")
        .set_modified(time)
        .expect("set the modification time");
}

#[test]
fn snapshot_date_quirk_is_the_modification_time_of_the_file_so_a_copy_gets_a_new_date() {
    let tmp = tempfile::tempdir().expect("tmp");
    let original = gold(tmp.path(), 2, 3);
    let fixed = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
    set_modified(&original, fixed);

    let first = status_of(&original);
    assert_eq!(first.snapshot_date, DateTime::<Utc>::from(fixed));
    assert_eq!(
        first.snapshot_date.to_rfc3339(),
        "2023-11-14T22:13:20+00:00"
    );

    // A copy made by reading and writing, which gives it the time of the copy
    // (`fs::copy` keeps the time of the source on Windows and not elsewhere).
    // `dvc pull` and a plain file copy change the date the same way.
    let copy = tmp.path().join("copy.sqlite");
    fs::write(&copy, fs::read(&original).expect("read")).expect("write the copy");
    let second = status_of(&copy);
    assert_eq!(second.sha256, first.sha256, "the same bytes");
    assert_ne!(second.snapshot_date, first.snapshot_date);
    assert!(
        (Utc::now() - second.snapshot_date).num_seconds().abs() < 120,
        "the date of the copy is about now: {}",
        second.snapshot_date
    );
}

// ─── Hash ───────────────────────────────────────────────────────────────────

#[test]
fn hash_is_stable_across_calls_and_changes_when_one_byte_changes() {
    let tmp = tempfile::tempdir().expect("tmp");
    let path = gold(tmp.path(), 2, 3);

    let first = {
        let referentiel = Referentiel::open(&path).expect("open");
        let a = referentiel.status().expect("status");
        let b = referentiel.status().expect("status again");
        assert_eq!(a, b, "two calls on the same file give the same status");
        a
    };

    // One bit of the "file change counter" of the SQLite header (bytes 24-27):
    // SQLite does not care about its value when it only reads.
    let mut bytes = fs::read(&path).expect("read");
    bytes[27] ^= 0x01;
    fs::write(&path, bytes).expect("write");

    let second = status_of(&path);
    assert_ne!(
        second.sha256, first.sha256,
        "the hash covers the whole file"
    );
    assert_eq!(second.source_count, first.source_count);
    assert_eq!(second.model_count, first.model_count);
}

// ─── Broken files ───────────────────────────────────────────────────────────

#[test]
fn open_quirk_a_file_that_is_not_sqlite_opens_and_only_status_fails() {
    let tmp = tempfile::tempdir().expect("tmp");
    let path = tmp.path().join("not-a-database.sqlite");
    fs::write(
        &path,
        b"this is not a SQLite database, but it is more than 100 bytes long, ".repeat(4),
    )
    .expect("write");

    // `open` documents `Sqlite` "si SQLite refuse l'ouverture": SQLite opens the
    // file lazily, so the error comes later.
    let referentiel = Referentiel::open(&path).expect("open does not read the file");
    let error = referentiel.status().expect_err("status fails");
    assert!(matches!(error, ReferentielError::Sqlite(_)), "{error:?}");
    assert!(error.to_string().contains("not a database"), "{error}");
}

#[test]
fn status_fails_when_the_sources_table_is_missing_unlike_model_overview() {
    let tmp = tempfile::tempdir().expect("tmp");
    let path = tmp.path().join("no-sources.sqlite");
    let conn = Connection::open(&path).expect("create");
    conn.execute_batch("CREATE TABLE model_overview (id TEXT);")
        .expect("create the table");
    drop(conn);

    let error = Referentiel::open(&path)
        .expect("open")
        .status()
        .expect_err("sources is required");
    assert!(matches!(error, ReferentielError::Sqlite(_)), "{error:?}");
    assert!(
        error.to_string().contains("no such table: sources"),
        "{error}"
    );
}

#[test]
fn status_counts_the_rows_of_model_overview_whatever_its_columns() {
    let tmp = tempfile::tempdir().expect("tmp");
    let path = tmp.path().join("odd-columns.sqlite");
    let conn = Connection::open(&path).expect("create");
    conn.execute_batch(
        "CREATE TABLE sources (id TEXT);
         CREATE TABLE model_overview (unexpected INTEGER);
         INSERT INTO model_overview VALUES (1), (2), (3), (4);",
    )
    .expect("create");
    drop(conn);

    let status = status_of(&path);
    assert_eq!(status.source_count, 0);
    assert_eq!(status.model_count, 4);
}

#[test]
fn status_quirk_a_model_overview_that_cannot_be_read_counts_as_zero_models() {
    // A full-text table whose content table is missing exists in `sqlite_master`
    // but cannot be counted: `status` turns that error into "0 models".
    let tmp = tempfile::tempdir().expect("tmp");
    let path = tmp.path().join("unreadable-models.sqlite");
    let conn = Connection::open(&path).expect("create");
    conn.execute_batch(
        "CREATE TABLE sources (id TEXT);
         INSERT INTO sources VALUES ('a'), ('b');
         CREATE VIRTUAL TABLE model_overview USING fts5(name, content='missing_content');",
    )
    .expect("create");
    assert!(
        conn.query_row("SELECT COUNT(*) FROM model_overview", [], |r| r
            .get::<_, i64>(0))
            .is_err(),
        "the count of model_overview really fails"
    );
    drop(conn);

    let status = status_of(&path);
    assert_eq!(status.source_count, 2);
    assert_eq!(status.model_count, 0);
}
