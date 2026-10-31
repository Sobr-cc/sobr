//! Characterization tests of the `sobria-ingest` commands (SOB-222).
//!
//! They run the real binary on a temp data folder. The standard registry has
//! the data.gouv.fr URLs built in, so the binary is cut off from the network
//! instead: its environment points every proxy variable at a closed local
//! port, and each download fails at once (the downloader still retries three
//! times, which is why the tests that download take several seconds).
//!
//! Tests named `quirk` record behavior that differs from CLAUDE.md section 10
//! or `dvc.yaml`. SOB-123 is the issue that fixes `gold` and `--incremental`.

mod common;

use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    net::TcpListener,
    path::{Path, PathBuf},
    process::{Command, Output, Stdio},
    time::{Duration, Instant},
};

use common::{count, gold_sqlite, populate};

/// A local address nothing listens on.
fn closed_local_address() -> String {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let port = listener.local_addr().expect("address").port();
    drop(listener);
    format!("http://127.0.0.1:{port}")
}

/// Runs `sobria-ingest <args>` with `data_root` as its data folder and no
/// network.
fn ingest(data_root: &Path, args: &[&str]) -> Output {
    let dead = closed_local_address();
    let mut command = Command::new(env!("CARGO_BIN_EXE_sobria-ingest"));
    command
        .args(args)
        .current_dir(data_root)
        .env("SOBRIA_DATA_ROOT", data_root)
        .env_remove("SOBRIA_SEED")
        .env("RUST_LOG", "error")
        .env_remove("NO_PROXY")
        .env_remove("no_proxy");
    for name in [
        "HTTP_PROXY",
        "HTTPS_PROXY",
        "ALL_PROXY",
        "http_proxy",
        "https_proxy",
        "all_proxy",
    ] {
        command.env(name, &dead);
    }

    let mut out_file = tempfile::tempfile().expect("stdout file");
    let mut err_file = tempfile::tempfile().expect("stderr file");
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::from(out_file.try_clone().expect("clone")))
        .stderr(Stdio::from(err_file.try_clone().expect("clone")))
        .spawn()
        .expect("run sobria-ingest");

    // If the proxy variables ever stopped cutting the network, the binary would
    // start downloads of several GB: stop it and fail instead of waiting.
    let deadline = Instant::now() + Duration::from_secs(90);
    let status = loop {
        if let Some(status) = child.try_wait().expect("wait") {
            break status;
        }
        if Instant::now() > deadline {
            let _ = child.kill();
            let _ = child.wait();
            panic!("`sobria-ingest {args:?}` ran for 90 s: the proxy variables did not cut the network");
        }
        std::thread::sleep(Duration::from_millis(50));
    };

    let read = |file: &mut File| {
        let mut bytes = Vec::new();
        file.seek(SeekFrom::Start(0)).expect("seek");
        file.read_to_end(&mut bytes).expect("read");
        bytes
    };
    Output {
        status,
        stdout: read(&mut out_file),
        stderr: read(&mut err_file),
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

/// The first file under `dir` (at any depth) whose name ends with `suffix`.
fn find_file(dir: &Path, suffix: &str) -> PathBuf {
    search(dir, suffix)
        .unwrap_or_else(|| panic!("no file ending with {suffix} under {}", dir.display()))
}

fn search(dir: &Path, suffix: &str) -> Option<PathBuf> {
    for entry in std::fs::read_dir(dir).ok()? {
        let path = entry.ok()?.path();
        if path.is_dir() {
            if let Some(found) = search(&path, suffix) {
                return Some(found);
            }
        } else if path.to_string_lossy().ends_with(suffix) {
            return Some(path);
        }
    }
    None
}

#[test]
fn pipeline_run_with_an_unknown_source_is_refused_before_any_download() {
    let tmp = tempfile::tempdir().expect("tmp");
    let out = ingest(tmp.path(), &["pipeline", "run", "--source", "nope"]);

    assert_eq!(out.status.code(), Some(1));
    let err = stderr(&out);
    assert!(err.contains("source inconnue: 'nope'"), "{err}");
    assert!(err.contains("comparia, rte-iris"), "{err}");
    assert!(
        !tmp.path().join("copper").exists(),
        "nothing was downloaded"
    );
}

#[test]
fn validate_without_any_snapshot_says_so_and_succeeds() {
    let tmp = tempfile::tempdir().expect("tmp");
    let out = ingest(tmp.path(), &["validate"]);

    assert_eq!(out.status.code(), Some(0));
    assert!(
        stdout(&out).contains("Aucun snapshot Copper trouvé sous"),
        "{}",
        stdout(&out)
    );
}

#[tokio::test]
async fn validate_accepts_untouched_snapshots() {
    let tmp = tempfile::tempdir().expect("tmp");
    populate(tmp.path()).await;

    let out = ingest(tmp.path(), &["validate"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("2 manifest(s) inspecté(s), 0 KO."),
        "{}",
        stdout(&out)
    );
}

#[tokio::test]
async fn validate_reports_a_corrupted_copper_file() {
    let tmp = tempfile::tempdir().expect("tmp");
    populate(tmp.path()).await;
    let csv = find_file(&tmp.path().join("copper"), "consommation_iris.csv");
    let mut bytes = std::fs::read(&csv).expect("csv");
    bytes.extend_from_slice(b"tampered");
    std::fs::write(&csv, bytes).expect("rewrite csv");

    let out = ingest(tmp.path(), &["validate"]);
    assert_eq!(out.status.code(), Some(1));
    let text = stdout(&out);
    assert!(text.contains("KO   "), "{text}");
    assert!(text.contains("2 manifest(s) inspecté(s), 1 KO."), "{text}");
    assert!(
        stderr(&out).contains("validate: 1 manifest(s) corrompu(s)"),
        "{}",
        stderr(&out)
    );
}

#[tokio::test]
async fn validate_quirk_only_rehashes_copper_so_broken_silver_and_missing_gold_pass() {
    let tmp = tempfile::tempdir().expect("tmp");
    populate(tmp.path()).await;
    // main.rs:201: nothing but the Copper manifests is checked, although
    // dvc.yaml:78-80 promises lineage, schema and row-count checks.
    for entity in ["comparia_conversations", "rte_iris_consommation"] {
        let silver = find_file(&tmp.path().join("silver"), &format!("{entity}-v1.parquet"));
        std::fs::write(&silver, b"not a parquet file").expect("break silver");
    }
    std::fs::remove_dir_all(tmp.path().join("gold")).expect("remove gold");

    let out = ingest(tmp.path(), &["validate"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("2 manifest(s) inspecté(s), 0 KO."),
        "{}",
        stdout(&out)
    );
}

#[tokio::test]
async fn silver_promotes_the_copper_on_disk_without_the_network() {
    let tmp = tempfile::tempdir().expect("tmp");
    populate(tmp.path()).await;
    std::fs::remove_dir_all(tmp.path().join("silver")).expect("remove silver");

    let out = ingest(tmp.path(), &["silver", "--all"]);
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    let text = stdout(&out);
    assert!(text.contains("[copper (réhydraté)] 2 OK, 0 KO"), "{text}");
    assert!(text.contains("[silver] 2 OK, 0 KO"), "{text}");
    assert!(find_file(
        &tmp.path().join("silver"),
        "comparia_conversations-v1.parquet"
    )
    .exists());
    assert!(find_file(
        &tmp.path().join("silver"),
        "rte_iris_consommation-v1.parquet"
    )
    .exists());
}

#[test]
fn silver_without_any_copper_fails_and_points_to_the_copper_step() {
    let tmp = tempfile::tempdir().expect("tmp");
    let out = ingest(tmp.path(), &["silver", "--all"]);

    assert_eq!(out.status.code(), Some(1));
    assert!(
        stdout(&out).contains("[copper (réhydraté)] 0 OK, 2 KO"),
        "{}",
        stdout(&out)
    );
    assert!(
        stderr(&out).contains("silver: au moins une source n'a pas de snapshot Copper exploitable"),
        "{}",
        stderr(&out)
    );
}

#[tokio::test]
async fn gold_quirk_runs_the_whole_pipeline_and_replaces_gold_when_every_download_fails() {
    let tmp = tempfile::tempdir().expect("tmp");
    populate(tmp.path()).await;
    assert_eq!(count(&gold_sqlite(tmp.path()), "silver_entities"), 4);

    // main.rs:190-193: `gold` is the full pipeline, so it starts with Copper
    // (dvc.yaml:46-50 says the gold stage depends on `data/silver` only).
    let out = ingest(tmp.path(), &["gold"]);
    let text = stdout(&out);
    assert!(text.contains("[copper] 0 OK, 2 KO"), "{text}");
    assert!(text.contains("[silver] 0 OK, 2 KO"), "{text}");
    // The command still succeeds ...
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
    // ... and the Gold it wrote has no Silver entity, although the Silver files
    // of the first run are still on disk.
    let conn = gold_sqlite(tmp.path());
    assert_eq!(count(&conn, "sources"), 2);
    assert_eq!(count(&conn, "silver_entities"), 0);
    assert_eq!(count(&conn, "model_overview"), 0);
    assert!(find_file(
        &tmp.path().join("silver"),
        "comparia_conversations-v1.parquet"
    )
    .exists());
}

#[tokio::test]
async fn pipeline_run_quirk_incremental_downloads_again_although_copper_is_on_disk() {
    let tmp = tempfile::tempdir().expect("tmp");
    populate(tmp.path()).await;

    // main.rs:80,148: the flag is logged and never read by a source.
    let out = ingest(
        tmp.path(),
        &["pipeline", "run", "--source", "rte-iris", "--incremental"],
    );
    let text = stdout(&out);
    assert!(
        text.contains("[copper] 0 OK, 1 KO"),
        "a download was attempted: {text}"
    );
    assert_eq!(out.status.code(), Some(0), "{}", stderr(&out));
}
