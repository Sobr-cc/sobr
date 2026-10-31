//! Pins that `sobria-ingest` and `sobria-geoloc` describe the same territory JSON (SOB-189).
//!
//! Both sides read the samples committed in `sobria-geoloc/tests/fixtures`. A field renamed,
//! added or dropped on one side only changes what that side writes, and a test fails.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde_json::Value;
use sobria_ingest::sources::territoire_fr::{
    write_artifact_json, RteMixArtifact as IngestRteMix, TerritoireFrArtifact as IngestTerritoire,
};

const TERRITOIRE: &str = "territoire_fr.sample.json";
const RTE_MIX: &str = "rte_mix_fr.sample.json";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../sobria-geoloc/tests/fixtures")
        .join(name)
}

fn sample(name: &str) -> Value {
    serde_json::from_str(&std::fs::read_to_string(fixture(name)).unwrap()).unwrap()
}

/// Every key path of a JSON value, arrays folded into `[]` (`industrial_sites[].lat`).
fn key_paths(value: &Value) -> BTreeSet<String> {
    fn walk(value: &Value, path: &str, out: &mut BTreeSet<String>) {
        match value {
            Value::Object(map) => {
                for (key, child) in map {
                    let child_path = if path.is_empty() {
                        key.clone()
                    } else {
                        format!("{path}.{key}")
                    };
                    out.insert(child_path.clone());
                    walk(child, &child_path, out);
                }
            },
            Value::Array(items) => {
                for item in items {
                    walk(item, &format!("{path}[]"), out);
                }
            },
            _ => {},
        }
    }
    let mut out = BTreeSet::new();
    walk(value, "", &mut out);
    out
}

#[test]
fn territoire_sample_has_every_field_of_the_ingest_and_geoloc_structs() {
    let sample = sample(TERRITOIRE);
    let ingest: IngestTerritoire = serde_json::from_value(sample.clone()).unwrap();
    let geoloc: sobria_geoloc::TerritoireFrArtifact =
        serde_json::from_value(sample.clone()).unwrap();

    let ingest_json = serde_json::to_value(&ingest).unwrap();
    let geoloc_json = serde_json::to_value(&geoloc).unwrap();

    assert_eq!(key_paths(&ingest_json), key_paths(&sample), "ingest fields");
    assert_eq!(key_paths(&geoloc_json), key_paths(&sample), "geoloc fields");
    assert_eq!(ingest_json, sample, "ingest values");
    assert_eq!(geoloc_json, sample, "geoloc values");
}

#[test]
fn rte_mix_sample_has_every_field_of_the_ingest_and_geoloc_structs() {
    let sample = sample(RTE_MIX);
    let ingest: IngestRteMix = serde_json::from_value(sample.clone()).unwrap();
    let geoloc: sobria_geoloc::RteMixArtifact = serde_json::from_value(sample.clone()).unwrap();

    let ingest_json = serde_json::to_value(&ingest).unwrap();
    let geoloc_json = serde_json::to_value(&geoloc).unwrap();

    assert_eq!(key_paths(&ingest_json), key_paths(&sample), "ingest fields");
    assert_eq!(key_paths(&geoloc_json), key_paths(&sample), "geoloc fields");
    assert_eq!(ingest_json, sample, "ingest values");
    assert_eq!(geoloc_json, sample, "geoloc values");
}

#[test]
fn territoire_file_written_by_ingest_is_loaded_by_geoloc() {
    let ingest: IngestTerritoire = serde_json::from_value(sample(TERRITOIRE)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("territoire_fr.json");

    write_artifact_json(&ingest, &path).unwrap();

    let loaded = sobria_geoloc::load_territoire_fr(&path).unwrap();
    assert_eq!(serde_json::to_value(&loaded).unwrap(), sample(TERRITOIRE));
}

#[test]
fn rte_mix_file_written_by_ingest_is_loaded_by_geoloc() {
    let ingest: IngestRteMix = serde_json::from_value(sample(RTE_MIX)).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("rte_mix_fr.json");

    write_artifact_json(&ingest, &path).unwrap();

    let loaded = sobria_geoloc::load_rte_mix(&path).unwrap();
    assert_eq!(serde_json::to_value(&loaded).unwrap(), sample(RTE_MIX));
}
