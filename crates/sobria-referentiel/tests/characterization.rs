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
use rusqlite::{Connection, ErrorCode};
use sha2::{Digest, Sha256};
use sobria_referentiel::{
    load, Referentiel, ReferentielError, ReferentielStatus, DEFAULT_REFERENTIEL_PATH,
    REFERENTIEL_PATH_ENV,
};

/// A database file made of this SQL, in one transaction.
fn db(dir: &Path, file: &str, sql: &str) -> PathBuf {
    let path = dir.join(file);
    Connection::open(&path)
        .expect("create the database")
        .execute_batch(&format!("BEGIN; {sql}; COMMIT;"))
        .expect("run the SQL");
    path
}

/// A Gold file: 3 sources and 2 models.
fn gold(dir: &Path) -> PathBuf {
    db(
        dir,
        "referentiel.sqlite",
        "CREATE TABLE sources (id TEXT PRIMARY KEY, name TEXT NOT NULL);
         CREATE TABLE model_overview (id TEXT PRIMARY KEY, name TEXT);
         INSERT INTO sources VALUES ('a', 'A'), ('b', 'B'), ('c', 'C');
         INSERT INTO model_overview VALUES ('m1', 'M1'), ('m2', 'M2')",
    )
}

fn status_of(path: &Path) -> ReferentielStatus {
    Referentiel::open(path)
        .expect("open")
        .status()
        .expect("status")
}

// ─── Default path ───────────────────────────────────────────────────────────

/// Set in a child process (see [`run_child`]): the file where it writes "ran"
/// once its assertions have passed.
const CHILD: &str = "SOBRIA_REFERENTIEL_TEST_CHILD";

/// Runs the test `name` of this binary again in `folder`, without
/// `SOBRIA_REFERENTIEL_PATH`. The test notices the variable `CHILD` and plays
/// its scenario instead, asserting in place. The sentinel file shows that it
/// really ran: a mistyped `name` matches no test, and the harness exits 0.
fn run_child(name: &str, folder: &Path, sentinel: &Path) {
    assert!(
        std::env::var_os(CHILD).is_none(),
        "run_child called from a child"
    );
    let output = Command::new(std::env::current_exe().expect("this test binary"))
        .args(["--exact", name])
        .env(CHILD, sentinel)
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
    assert_eq!(
        fs::read_to_string(sentinel).ok().as_deref(),
        Some("ran"),
        "the child did not run its scenario: is `{name}` still the name of the test?"
    );
}

#[test]
fn load_quirk_without_the_variable_the_default_path_is_relative_to_the_current_folder() {
    if let Some(sentinel) = std::env::var_os(CHILD) {
        let referentiel = load().expect("the default file exists in the current folder");
        assert_eq!(referentiel.path(), Path::new(DEFAULT_REFERENTIEL_PATH));
        let status = referentiel.status().expect("status");
        // The documentation of `ReferentielStatus::path` says "chemin absolu": it
        // is the path as given, here the relative default.
        assert_eq!(status.path, DEFAULT_REFERENTIEL_PATH);
        assert_eq!(status.source_count, 3);
        fs::write(sentinel, "ran").expect("write the sentinel");
        return;
    }

    // `data/gold/referentiel.sqlite` under a folder that is not the folder of
    // the test.
    let tmp = tempfile::tempdir().expect("tmp");
    let gold_dir = tmp
        .path()
        .join(DEFAULT_REFERENTIEL_PATH)
        .parent()
        .expect("a folder")
        .to_path_buf();
    fs::create_dir_all(&gold_dir).expect("create data/gold");
    gold(&gold_dir);

    run_child(
        "load_quirk_without_the_variable_the_default_path_is_relative_to_the_current_folder",
        tmp.path(),
        &tmp.path().join("child-done"),
    );
}

#[test]
fn load_without_the_variable_does_not_find_the_default_path_from_another_folder() {
    if let Some(sentinel) = std::env::var_os(CHILD) {
        let error = load().err().expect("nothing at the default path here");
        assert!(
            matches!(&error, ReferentielError::NotFound(p) if p == Path::new(DEFAULT_REFERENTIEL_PATH)),
            "{error:?}"
        );
        fs::write(sentinel, "ran").expect("write the sentinel");
        return;
    }

    let empty = tempfile::tempdir().expect("tmp");
    run_child(
        "load_without_the_variable_does_not_find_the_default_path_from_another_folder",
        empty.path(),
        &empty.path().join("child-done"),
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
    let original = gold(tmp.path());
    set_modified(
        &original,
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000),
    );

    let first = status_of(&original);
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
    assert!(second.snapshot_date > first.snapshot_date);
    let modified = fs::metadata(&copy)
        .and_then(|m| m.modified())
        .expect("modification time of the copy");
    assert_eq!(second.snapshot_date, DateTime::<Utc>::from(modified));
}

// ─── Hash ───────────────────────────────────────────────────────────────────

#[test]
fn hash_is_the_sha256_of_the_whole_file_and_changes_when_one_byte_changes() {
    // A file of about 200 KB, so that the hash is read in several chunks of
    // 64 KB: the big row is the last thing written, and `COUNT(*)` does not read
    // its overflow pages, so `status` still works after the last byte changes.
    let tmp = tempfile::tempdir().expect("tmp");
    let path = db(
        tmp.path(),
        "big.sqlite",
        "CREATE TABLE sources (id TEXT PRIMARY KEY, name TEXT NOT NULL);
         INSERT INTO sources VALUES ('a', 'A');
         INSERT INTO sources VALUES ('big', hex(zeroblob(100000)))",
    );
    assert!(fs::metadata(&path).expect("metadata").len() > 128 * 1024);

    let first = status_of(&path);
    assert_eq!(first, status_of(&path), "two calls give the same status");
    let sha256_of_the_file = |p: &Path| format!("{:x}", Sha256::digest(fs::read(p).expect("read")));
    assert_eq!(first.sha256, sha256_of_the_file(&path));

    let mut bytes = fs::read(&path).expect("read");
    *bytes.last_mut().expect("a non-empty file") ^= 0x01;
    fs::write(&path, bytes).expect("write");

    let second = status_of(&path);
    assert_ne!(second.sha256, first.sha256, "the hash covers the last byte");
    assert_eq!(second.sha256, sha256_of_the_file(&path));
    assert_eq!(second.source_count, first.source_count);
}

// ─── Broken files ───────────────────────────────────────────────────────────

#[test]
fn open_quirk_a_file_that_is_not_sqlite_opens_and_only_status_fails() {
    let tmp = tempfile::tempdir().expect("tmp");
    let path = tmp.path().join("not-a-database.sqlite");
    fs::write(&path, vec![b'x'; 200]).expect("write");

    // `open` documents `Sqlite` "si SQLite refuse l'ouverture": SQLite opens the
    // file lazily, so the error comes later.
    let referentiel = Referentiel::open(&path).expect("open does not read the file");
    let error = referentiel.status().expect_err("status fails");
    assert!(
        matches!(
            &error,
            ReferentielError::Sqlite(rusqlite::Error::SqliteFailure(e, _))
                if e.code == ErrorCode::NotADatabase
        ),
        "{error:?}"
    );
}

#[test]
fn status_fails_when_the_sources_table_is_missing_unlike_model_overview() {
    let tmp = tempfile::tempdir().expect("tmp");
    let path = db(
        tmp.path(),
        "no-sources.sqlite",
        "CREATE TABLE model_overview (id TEXT)",
    );

    let error = Referentiel::open(&path)
        .expect("open")
        .status()
        .expect_err("sources is required");
    assert!(matches!(error, ReferentielError::Sqlite(_)), "{error:?}");
    assert!(error.to_string().contains("sources"), "{error}");
}

#[test]
fn status_counts_the_rows_of_model_overview_whatever_its_columns() {
    let tmp = tempfile::tempdir().expect("tmp");
    let path = db(
        tmp.path(),
        "odd-columns.sqlite",
        "CREATE TABLE sources (id TEXT);
         CREATE TABLE model_overview (unexpected INTEGER);
         INSERT INTO model_overview VALUES (1), (2), (3), (4)",
    );

    let status = status_of(&path);
    assert_eq!(status.source_count, 0);
    assert_eq!(status.model_count, 4);
}

#[test]
fn status_quirk_a_model_overview_that_cannot_be_read_counts_as_zero_models() {
    // A full-text table whose content table is missing exists in `sqlite_master`
    // but cannot be counted: `status` turns that error into "0 models".
    let tmp = tempfile::tempdir().expect("tmp");
    let path = db(
        tmp.path(),
        "unreadable-models.sqlite",
        "CREATE TABLE sources (id TEXT);
         INSERT INTO sources VALUES ('a'), ('b');
         CREATE VIRTUAL TABLE model_overview USING fts5(name, content='missing_content')",
    );
    let count = Connection::open(&path).expect("open").query_row(
        "SELECT COUNT(*) FROM model_overview",
        [],
        |r| r.get::<_, i64>(0),
    );
    assert!(count.is_err(), "the count of model_overview really fails");

    let status = status_of(&path);
    assert_eq!(status.source_count, 2);
    assert_eq!(status.model_count, 0);
}
