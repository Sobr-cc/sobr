//! Characterization tests of the Copper -> Silver -> Gold pipeline (SOB-222).
//!
//! They run the sources against a local stand-in for data.gouv.fr and record
//! what the pipeline writes today: golden files for the Silver rows, the Gold
//! SQLite and the datasheet, and the behaviors the codebase audit flagged
//! (tests named `quirk`). Nothing here fixes them: SOB-123 and SOB-21 do.
//!
//! To rewrite the golden files, see `test-support/golden.rs`
//! (`SOBRIA_UPDATE_GOLDEN=1`).

mod common;

use std::path::Path;

use common::{
    context, count, gold_sqlite, golden::Goldens, only_snapshot, parquet_dump, populate,
    populate_from, silver_path, sqlite_dump, Upstream, CSV_RTE, GEOJSON_EMPTY, GEOJSON_RTE, SILVER,
};
use serde_json::Value;
use sobria_ingest::{
    hash,
    layer::{CopperSnapshot, DataLayer},
    CopperManifest,
};

#[tokio::test]
async fn silver_files_of_both_sources_are_pinned() {
    let tmp = tempfile::tempdir().expect("tmp");
    populate_from(&Upstream::start_without_geometries().await, tmp.path()).await;

    let mut goldens = Goldens::new();
    for (source, entity, copper_file) in SILVER {
        let mut dump = parquet_dump(&silver_path(tmp.path(), source, entity));
        // The hash is masked in the golden file, so check it here: it must be
        // the hash of the Copper file the rows come from.
        let copper_hash = hash::sha256_file(&only_snapshot(tmp.path(), source).join(copper_file))
            .await
            .expect("hash the Copper file");
        let rows = dump["rows"].as_array().expect("rows");
        assert!(!rows.is_empty(), "{entity}: no row");
        for row in rows {
            assert_eq!(row["_copper_sha256"], copper_hash, "{entity}: lineage hash");
        }

        common::mask_volatile(&mut dump, tmp.path());
        for row in dump["rows"].as_array().expect("rows") {
            assert_eq!(row["_ingested_at"], "<time>", "{entity}");
            assert_eq!(row["_copper_sha256"], "<sha256>", "{entity}");
        }
        goldens.assert(&format!("silver_{entity}"), &dump);
    }
}

#[tokio::test]
async fn gold_referentiel_and_datasheet_are_pinned() {
    let tmp = tempfile::tempdir().expect("tmp");
    populate_from(&Upstream::start_without_geometries().await, tmp.path()).await;
    let conn = gold_sqlite(tmp.path());

    // The hashes are masked in the golden files, so check here that each
    // lineage row carries the hash of the Copper file it names.
    let lineage: Vec<(String, String, String)> = conn
        .prepare("SELECT copper_sha256, source_id, file_name FROM lineage")
        .expect("prepare")
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .expect("query")
        .map(|r| r.expect("row"))
        .collect();
    assert_eq!(lineage.len(), 4);
    for (sha256, source, file) in lineage {
        let actual = hash::sha256_file(&only_snapshot(tmp.path(), &source).join(&file))
            .await
            .expect("hash the Copper file");
        assert_eq!(sha256, actual, "{source}/{file}");
    }

    let mut goldens = Goldens::new();
    goldens.assert("gold_referentiel", &sqlite_dump(&conn, tmp.path()));

    let text = std::fs::read_to_string(tmp.path().join("gold").join("datasheet.jsonld"))
        .expect("datasheet");
    let mut datasheet: Value = serde_json::from_str(&text).expect("datasheet JSON");
    common::mask_volatile(&mut datasheet, tmp.path());
    goldens.assert("gold_datasheet", &datasheet);
}

#[tokio::test]
async fn gold_quirk_four_tables_stay_empty_and_conversation_counts_stay_zero() {
    let tmp = tempfile::tempdir().expect("tmp");
    populate(tmp.path()).await;

    let conn = gold_sqlite(tmp.path());
    // `build_referentiel_sqlite` creates these tables and no source fills them.
    for table in [
        "scenario_inputs",
        "time_series_mix",
        "comparison_matrix",
        "vendor_disclosures",
    ] {
        assert_eq!(count(&conn, table), 0, "{table}");
    }
    // `populate_model_overview` inserts every model with a fixed 0.
    assert_eq!(count(&conn, "model_overview"), 3);
    let total: i64 = conn
        .query_row("SELECT SUM(n_conversations) FROM model_overview", [], |r| {
            r.get(0)
        })
        .expect("sum");
    assert_eq!(total, 0);
}

#[tokio::test]
async fn gold_quirk_a_run_on_one_source_rebuilds_gold_from_that_source_alone() {
    let tmp = tempfile::tempdir().expect("tmp");
    let upstream = Upstream::start().await;
    populate_from(&upstream, tmp.path()).await;
    let linked = {
        let conn = gold_sqlite(tmp.path());
        assert_eq!(count(&conn, "sources"), 2);
        assert_eq!(count(&conn, "silver_entities"), 4);
        assert_eq!(count(&conn, "lineage"), 4);
        let linked = count(&conn, "datacenter_iris_link");
        assert!(linked > 0, "the IRIS file has features");
        linked
    };

    // `pipeline run --source comparia` builds exactly this registry.
    upstream
        .comparia_registry()
        .run_full_pipeline(&context(tmp.path()))
        .await
        .expect("one-source pipeline");
    let conn = gold_sqlite(tmp.path());
    let sources: Vec<String> = conn
        .prepare("SELECT id FROM sources")
        .expect("prepare")
        .query_map([], |r| r.get(0))
        .expect("query")
        .map(|r| r.expect("row"))
        .collect();
    assert_eq!(sources, ["comparia"], "rte-iris disappeared from `sources`");
    assert_eq!(count(&conn, "silver_entities"), 3);
    assert_eq!(count(&conn, "lineage"), 3);
    assert_eq!(count(&conn, "model_overview"), 3);
    // The link table is computed from the RTE Copper left on disk, so it stays.
    assert_eq!(count(&conn, "datacenter_iris_link"), linked);
    // The RTE Silver file is still there, but no longer in Gold.
    assert!(silver_path(tmp.path(), "rte-iris", "rte_iris_consommation").exists());
}

#[tokio::test]
async fn gold_quirk_a_run_where_every_download_fails_replaces_gold_with_an_empty_one() {
    let tmp = tempfile::tempdir().expect("tmp");
    let upstream = Upstream::start().await;
    let ctx = context(tmp.path());
    let registry = upstream.registry();

    registry.run_full_pipeline(&ctx).await.expect("first run");
    let linked = {
        let conn = gold_sqlite(tmp.path());
        assert_eq!(count(&conn, "silver_entities"), 4);
        count(&conn, "datacenter_iris_link")
    };

    // data.gouv.fr answers 404 for everything (not retried by the downloader).
    upstream.refuse_everything().await;
    let report = registry
        .run_full_pipeline(&ctx)
        .await
        .expect("the pipeline itself still returns Ok");
    assert_eq!(report.failed_sources(), ["comparia", "rte-iris"]);
    assert!(
        report.gold_artifacts.is_some(),
        "Gold is assembled although no source succeeded"
    );

    let conn = gold_sqlite(tmp.path());
    assert_eq!(
        count(&conn, "sources"),
        2,
        "the source list comes from the registry"
    );
    assert_eq!(count(&conn, "silver_entities"), 0);
    assert_eq!(count(&conn, "lineage"), 0);
    assert_eq!(count(&conn, "model_overview"), 0);
    // The good Silver of the first run is still on disk, and unused.
    assert!(silver_path(tmp.path(), "comparia", "comparia_conversations").exists());
    // The links come from the Copper of the first run, which is also still there.
    assert_eq!(count(&conn, "datacenter_iris_link"), linked);
}

/// `true` if some file under `dir` holds exactly `content`.
fn holds_file_with(dir: &Path, content: &[u8]) -> bool {
    std::fs::read_dir(dir)
        .expect("read_dir")
        .map(|entry| entry.expect("entry").path())
        .any(|path| {
            if path.is_dir() {
                holds_file_with(&path, content)
            } else {
                std::fs::read(&path).is_ok_and(|bytes| bytes == content)
            }
        })
}

fn file_hash(snapshot: &CopperSnapshot, name: &str) -> String {
    snapshot
        .files
        .iter()
        .find(|f| f.file_name == name)
        .expect("file in the snapshot")
        .file_sha256
        .clone()
}

#[tokio::test]
async fn copper_quirk_a_second_run_on_the_same_day_overwrites_the_snapshot() {
    let upstream = Upstream::start().await;
    let source = upstream.rte_iris();
    let second_csv = b"code_iris,annee\n111,2024\n";

    // The snapshot folder is named after the UTC day: if midnight passes between
    // the two runs they land in two folders, and the pair is run again.
    for _ in 0..3 {
        let tmp = tempfile::tempdir().expect("tmp");
        let ctx = context(tmp.path());

        upstream.replace_rte(CSV_RTE, GEOJSON_RTE).await;
        let first = source.ingest_copper(&ctx).await.expect("first run");
        upstream.replace_rte(second_csv, GEOJSON_EMPTY).await;
        let second = source.ingest_copper(&ctx).await.expect("second run");
        if first.path != second.path {
            continue;
        }

        let snapshot = only_snapshot(tmp.path(), "rte-iris");
        assert_eq!(snapshot, second.path, "one folder for the day");
        assert_eq!(
            std::fs::read(snapshot.join("consommation_iris.csv")).expect("csv"),
            second_csv
        );
        assert_ne!(
            file_hash(&first, "consommation_iris.csv"),
            file_hash(&second, "consommation_iris.csv")
        );
        let manifest = CopperManifest::load(&snapshot.join("manifest.json"))
            .await
            .expect("manifest");
        let entry = manifest
            .files
            .iter()
            .find(|f| f.name == "consommation_iris.csv")
            .expect("csv in the manifest");
        assert_eq!(entry.sha256, file_hash(&second, "consommation_iris.csv"));
        assert!(manifest.verify_files(&snapshot).await.is_ok());
        // Nothing keeps the first run: its file is no longer anywhere on disk.
        assert!(!holds_file_with(tmp.path(), CSV_RTE.as_bytes()));
        return;
    }
    panic!("the two runs landed on different days three times in a row");
}

#[tokio::test]
async fn incremental_quirk_the_flag_does_not_prevent_a_second_download() {
    let tmp = tempfile::tempdir().expect("tmp");
    let upstream = Upstream::start().await;
    let mut ctx = context(tmp.path());
    ctx.incremental = true;
    let registry = upstream.registry();

    registry.run_copper(&ctx).await;
    let first = upstream.request_count().await;
    assert!(
        first >= 5,
        "3 ComparIA files, the RTE CSV and the RTE GeoJSON"
    );

    registry.run_copper(&ctx).await;
    assert_eq!(
        upstream.request_count().await,
        2 * first,
        "nothing changed upstream, and the same files are downloaded again"
    );
}

#[tokio::test]
async fn silver_quirk_two_identical_runs_differ_by_the_run_time_alone() {
    let tmp = tempfile::tempdir().expect("tmp");
    let upstream = Upstream::start().await;
    let ctx = context(tmp.path());
    let registry = upstream.comparia_registry();
    let file = silver_path(tmp.path(), "comparia", "comparia_conversations");

    let copper = registry.run_copper(&ctx).await;
    registry.run_silver(&ctx, &copper).await;
    let first = parquet_dump(&file);
    let copper = registry.run_copper(&ctx).await;
    registry.run_silver(&ctx, &copper).await;
    let second = parquet_dump(&file);

    // `_ingested_at` is the run time, so `dvc repro` cannot reproduce Silver
    // byte for byte (CLAUDE.md section 3bis, rule 8).
    assert_ne!(
        first["rows"][0]["_ingested_at"],
        second["rows"][0]["_ingested_at"]
    );
    let without_run_time = |mut dump: Value| {
        for row in dump["rows"].as_array_mut().expect("rows") {
            row.as_object_mut().expect("row").remove("_ingested_at");
        }
        dump
    };
    assert_eq!(without_run_time(first), without_run_time(second));
}
