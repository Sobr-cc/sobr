//! Characterization tests of the Copper -> Silver -> Gold pipeline (SOB-222).
//!
//! They run both sources against a local stand-in for data.gouv.fr and record
//! what the pipeline writes today: golden files for the Silver rows, the Gold
//! SQLite and the datasheet, and the behaviors the codebase audit flagged
//! (tests named `quirk`). Nothing here fixes them: SOB-123 and SOB-21 do.
//!
//! To rewrite the golden files, see `test-support/golden.rs`
//! (`SOBRIA_UPDATE_GOLDEN=1`).

mod common;

use std::collections::HashSet;

use common::{
    context, count, gold_sqlite, golden::Goldens, mask_volatile, only_snapshot, parquet_dump,
    sqlite_dump_masked, Upstream, CSV_RTE, GEOJSON_EMPTY,
};
use serde_json::Value;
use sobria_ingest::{hash, layer::DataLayer, CopperManifest};

/// `(source, Silver entity, Copper file it comes from)`.
const SILVER: [(&str, &str, &str); 4] = [
    (
        "comparia",
        "comparia_conversations",
        "conversations.parquet",
    ),
    ("comparia", "comparia_votes", "votes.parquet"),
    ("comparia", "comparia_reactions", "reactions.parquet"),
    ("rte-iris", "rte_iris_consommation", "consommation_iris.csv"),
];

fn silver_path(data_root: &std::path::Path, source: &str, entity: &str) -> std::path::PathBuf {
    data_root
        .join("silver")
        .join(source)
        .join(format!("{entity}-v1.parquet"))
}

/// Number of distinct datacenters the Gold link table is built from.
fn datacenter_count() -> i64 {
    let ids: HashSet<&str> = sobria_geoloc::all_datacenters()
        .iter()
        .map(|dc| dc.id.as_str())
        .collect();
    i64::try_from(ids.len()).expect("datacenter count")
}

#[tokio::test]
async fn silver_files_of_both_sources_are_pinned() {
    let tmp = tempfile::tempdir().expect("tmp");
    let upstream = Upstream::start_without_geometries().await;
    upstream
        .registry()
        .run_full_pipeline(&context(tmp.path()))
        .await
        .expect("pipeline");

    let mut goldens = Goldens::new();
    for (source, entity, copper_file) in SILVER {
        let mut dump = parquet_dump(&silver_path(tmp.path(), source, entity));
        // The masked hash must still be the hash of the Copper file it came from.
        let copper_hash = hash::sha256_file(&only_snapshot(tmp.path(), source).join(copper_file))
            .await
            .expect("hash the Copper file");
        let rows = dump["rows"].as_array().expect("rows").len();
        assert!(rows > 0, "{entity}: no row");
        for row in dump["rows"].as_array().expect("rows") {
            assert_eq!(row["_copper_sha256"], copper_hash, "{entity}: lineage hash");
        }
        let masked = mask_volatile(&mut dump, tmp.path());
        assert_eq!(
            masked,
            2 * rows,
            "{entity}: `_ingested_at` and `_copper_sha256` are masked in every row"
        );
        goldens.assert(&format!("silver_{entity}"), &dump);
    }
}

#[tokio::test]
async fn gold_referentiel_and_datasheet_are_pinned() {
    let tmp = tempfile::tempdir().expect("tmp");
    let upstream = Upstream::start_without_geometries().await;
    upstream
        .registry()
        .run_full_pipeline(&context(tmp.path()))
        .await
        .expect("pipeline");

    let mut goldens = Goldens::new();

    let referentiel = sqlite_dump_masked(&gold_sqlite(tmp.path()), tmp.path());
    goldens.assert("gold_referentiel", &referentiel);

    let text = std::fs::read_to_string(tmp.path().join("gold").join("datasheet.jsonld"))
        .expect("datasheet");
    let mut datasheet: Value = serde_json::from_str(&text).expect("datasheet JSON");
    mask_volatile(&mut datasheet, tmp.path());
    goldens.assert("gold_datasheet", &datasheet);
}

#[tokio::test]
async fn gold_quirk_four_tables_stay_empty_and_conversation_counts_stay_zero() {
    let tmp = tempfile::tempdir().expect("tmp");
    let upstream = Upstream::start_without_geometries().await;
    upstream
        .registry()
        .run_full_pipeline(&context(tmp.path()))
        .await
        .expect("pipeline");

    let conn = gold_sqlite(tmp.path());
    // gold.rs:235,243,249,278: created, never filled by any source.
    for table in [
        "scenario_inputs",
        "time_series_mix",
        "comparison_matrix",
        "vendor_disclosures",
    ] {
        assert_eq!(count(&conn, table), 0, "{table}");
    }
    // gold.rs:458: the model rows are inserted with a fixed 0.
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
    let ctx = context(tmp.path());

    upstream
        .registry()
        .run_full_pipeline(&ctx)
        .await
        .expect("full pipeline");
    {
        let conn = gold_sqlite(tmp.path());
        assert_eq!(count(&conn, "sources"), 2);
        assert_eq!(count(&conn, "silver_entities"), 4);
        assert_eq!(count(&conn, "lineage"), 4);
        assert_eq!(count(&conn, "datacenter_iris_link"), datacenter_count());
    }

    // `pipeline run --source comparia` builds exactly this registry.
    upstream
        .comparia_registry()
        .run_full_pipeline(&ctx)
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
    assert_eq!(count(&conn, "datacenter_iris_link"), datacenter_count());
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
    assert_eq!(count(&gold_sqlite(tmp.path()), "silver_entities"), 4);

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
    assert_eq!(count(&conn, "datacenter_iris_link"), datacenter_count());
}

#[tokio::test]
async fn copper_quirk_a_second_run_on_the_same_day_overwrites_the_snapshot() {
    let upstream = Upstream::start().await;
    let source = upstream.rte_iris();
    let second_csv = b"code_iris,annee\n111,2024\n";

    // The snapshot folder is named after the UTC day: if midnight passes between
    // the two runs they land in two folders, and the pair is run again.
    for _ in 0..3 {
        upstream.server.reset().await;
        upstream.serve_defaults().await;
        let tmp = tempfile::tempdir().expect("tmp");
        let ctx = context(tmp.path());
        let first = source.ingest_copper(&ctx).await.expect("first run");

        upstream.server.reset().await;
        upstream.serve("/rte/csv", second_csv.to_vec()).await;
        upstream
            .serve("/rte/geojson", GEOJSON_EMPTY.as_bytes().to_vec())
            .await;
        let second = source.ingest_copper(&ctx).await.expect("second run");
        if first.path != second.path {
            continue;
        }

        let snapshot = only_snapshot(tmp.path(), "rte-iris");
        assert_eq!(snapshot, second.path, "one folder for the day");
        assert_eq!(
            std::fs::read(snapshot.join("consommation_iris.csv")).expect("csv"),
            second_csv,
            "the first run's file is gone"
        );
        assert_ne!(first.files[0].file_sha256, second.files[0].file_sha256);
        let manifest = CopperManifest::load(&snapshot.join("manifest.json"))
            .await
            .expect("manifest");
        assert_eq!(manifest.files[0].sha256, second.files[0].file_sha256);
        assert!(manifest.verify_files(&snapshot).await.is_ok());
        // Nothing keeps the first run: its content is no longer on disk anywhere.
        assert_ne!(
            std::fs::read(snapshot.join("consommation_iris.csv")).expect("csv"),
            CSV_RTE.as_bytes()
        );
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

    registry.run_full_pipeline(&ctx).await.expect("first run");
    assert_eq!(
        upstream.request_count().await,
        5,
        "3 ComparIA files, the RTE CSV and the RTE GeoJSON"
    );

    registry.run_full_pipeline(&ctx).await.expect("second run");
    assert_eq!(
        upstream.request_count().await,
        10,
        "nothing changed upstream, and everything is downloaded again"
    );
}

#[tokio::test]
async fn silver_quirk_two_identical_runs_write_different_files() {
    let tmp = tempfile::tempdir().expect("tmp");
    let upstream = Upstream::start().await;
    let ctx = context(tmp.path());
    let registry = upstream.registry();
    let file = silver_path(tmp.path(), "comparia", "comparia_conversations");

    registry.run_full_pipeline(&ctx).await.expect("first run");
    let first = hash::sha256_file(&file).await.expect("hash");
    registry.run_full_pipeline(&ctx).await.expect("second run");
    let second = hash::sha256_file(&file).await.expect("hash");

    // `_ingested_at` is the run time, so `dvc repro` cannot reproduce Silver
    // byte for byte (CLAUDE.md section 3bis, rule 8).
    assert_ne!(first, second);
}
