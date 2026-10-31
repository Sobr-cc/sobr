//! Pins that `sobria-ingest` and `sobria-geoloc` describe the same territory JSON (SOB-189).
//!
//! Both sides read the samples committed in `sobria-geoloc/tests/fixtures`. A field renamed,
//! added or dropped on one side only changes what that side writes, and a test fails.

use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

use serde::{de::DeserializeOwned, Serialize};
use serde_json::Value;
use sobria_ingest::sources::territoire_fr::{
    write_artifact_json, RteMixArtifact as IngestRteMix, TerritoireFrArtifact as IngestTerritoire,
};

const TERRITOIRE: &str = "territoire_fr.sample.json";
const RTE_MIX: &str = "rte_mix_fr.sample.json";

fn geoloc_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../sobria-geoloc")
}

fn sample(name: &str) -> Value {
    let text = std::fs::read_to_string(geoloc_dir().join("tests/fixtures").join(name)).unwrap();
    serde_json::from_str(&text).unwrap()
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

/// Reads the sample with `T`, writes it back, and expects the same fields and values.
fn assert_writes_the_sample<T: Serialize + DeserializeOwned>(sample: &Value, side: &str) {
    let written =
        serde_json::to_value(serde_json::from_value::<T>(sample.clone()).unwrap()).unwrap();

    assert_eq!(key_paths(&written), key_paths(sample), "{side} fields");
    assert_eq!(&written, sample, "{side} values");
}

#[test]
fn territoire_sample_has_every_field_of_the_ingest_and_geoloc_structs() {
    let sample = sample(TERRITOIRE);

    assert_writes_the_sample::<IngestTerritoire>(&sample, "ingest");
    assert_writes_the_sample::<sobria_geoloc::TerritoireFrArtifact>(&sample, "geoloc");
}

#[test]
fn rte_mix_sample_has_every_field_of_the_ingest_and_geoloc_structs() {
    let sample = sample(RTE_MIX);

    assert_writes_the_sample::<IngestRteMix>(&sample, "ingest");
    assert_writes_the_sample::<sobria_geoloc::RteMixArtifact>(&sample, "geoloc");
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

/// The shipped files were fetched earlier; `rte_mix_fr.json` also carries a hand-added
/// `_meta.recalibrated_at`, so only reading is checked, not a byte-for-byte round trip.
#[test]
fn the_shipped_files_are_read_by_the_ingest_structs() {
    let territoire = std::fs::read_to_string(geoloc_dir().join("data/territoire_fr.json")).unwrap();
    let rte_mix = std::fs::read_to_string(geoloc_dir().join("data/rte_mix_fr.json")).unwrap();

    let territoire: IngestTerritoire = serde_json::from_str(&territoire).unwrap();
    let rte_mix: IngestRteMix = serde_json::from_str(&rte_mix).unwrap();

    assert_eq!(territoire.regions.len(), 13);
    assert_eq!(territoire.industrial_sites.len(), 200);
    assert_eq!(rte_mix.mix.year, 2023);
}
