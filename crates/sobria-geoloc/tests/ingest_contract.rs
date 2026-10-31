//! Pins how `sobria-geoloc` reads the JSON files written by `sobria-ingest` (SOB-189).
//!
//! The samples in `tests/fixtures` have the shape of the ingest output. The test
//! `territoire_contract.rs` of `sobria-ingest` checks that the ingest structs write
//! exactly these files, so both sides are tied to the same samples.

use std::{
    fmt::Debug,
    path::{Path, PathBuf},
};

use serde::de::DeserializeOwned;
use serde_json::Value;
use sobria_geoloc::{
    load_rte_mix, load_territoire_fr, ArtifactMeta, IndustrialSite, RegionMeta, RteMixArtifact,
    RteMixMeta, RteMixSourceTotals, TerritoireFrArtifact,
};

const TERRITOIRE: &str = "territoire_fr.sample.json";
const RTE_MIX: &str = "rte_mix_fr.sample.json";
const SAMPLE_SHA256: &str = "deadbeef00000000000000000000000000000000000000000000000000000000";
const SAMPLE_FETCHED_AT: &str = "2026-01-01T00:00:00.000000001+00:00";

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn fixture_text(name: &str) -> String {
    std::fs::read_to_string(fixture(name)).unwrap()
}

fn expected_territoire() -> TerritoireFrArtifact {
    TerritoireFrArtifact {
        meta: ArtifactMeta {
            version: "1.0.0".into(),
            fetched_at: SAMPLE_FETCHED_AT.into(),
            source_url: "https://odre.opendatasoft.com/api/explore/v2.1/catalog/datasets/consommation-annuelle-par-iris/records?limit=100&offset=0".into(),
            source_sha256: SAMPLE_SHA256.into(),
            license: "Etalab 2.0".into(),
            notes: vec![
                "Top sites industriels par consommation électrique annuelle ODRÉ.".into(),
                "Records bruts paginés : 2, retenus (top 2) : 2, écartés (DROM, géoloc manquante, conso ≤ 0) : 0.".into(),
            ],
        },
        regions: vec![
            RegionMeta {
                region_iso: "FR-ARA".into(),
                name: "Auvergne-Rhône-Alpes".into(),
                insee_code: "84".into(),
                centroid_lat: 45.5,
                centroid_lon: 4.8,
                nuclear_share_pct: 74.5,
            },
            RegionMeta {
                region_iso: "FR-IDF".into(),
                name: "Île-de-France".into(),
                insee_code: "11".into(),
                centroid_lat: 48.7,
                centroid_lon: 2.6,
                nuclear_share_pct: 0.0,
            },
        ],
        industrial_sites: vec![
            IndustrialSite {
                code_iris: "262350104".into(),
                commune: "Pierrelatte".into(),
                commune_code: "26235".into(),
                department_code: "26".into(),
                department_label: "Drôme".into(),
                region_insee_code: "84".into(),
                region_iso: "FR-ARA".into(),
                lat: 44.342_138_061_6,
                lon: 4.712_078_560_5,
                consumption_mwh_elec: 2_450_882.008,
                consumption_mwh_gas_grtgaz: 0.0,
                consumption_mwh_gas_terega: 0.0,
                consumption_total_mwh: 2_450_882.008,
                pdl_count_elec: 5,
                pdl_count_gas: 0,
                pdl_total: 5,
                // SOB-122: 0 is the year bug (ingest reads a field the API does not return).
                // Also in the sample file and in all 200 shipped sites.
                year: 0,
            },
            IndustrialSite {
                code_iris: "751010101".into(),
                commune: "Paris 1er".into(),
                commune_code: "75101".into(),
                department_code: "75".into(),
                department_label: "Paris".into(),
                region_insee_code: "11".into(),
                region_iso: "FR-IDF".into(),
                lat: 48.86,
                lon: 2.34,
                consumption_mwh_elec: 12_345.6,
                consumption_mwh_gas_grtgaz: 200.5,
                consumption_mwh_gas_terega: 30.25,
                consumption_total_mwh: 12_576.35,
                pdl_count_elec: 3,
                pdl_count_gas: 2,
                pdl_total: 5,
                year: 2022,
            },
        ],
    }
}

fn expected_rte_mix() -> RteMixArtifact {
    RteMixArtifact {
        meta: RteMixMeta {
            version: "1.0.0".into(),
            fetched_at: SAMPLE_FETCHED_AT.into(),
            source_url: "https://odre.opendatasoft.com/api/explore/v2.1/catalog/datasets/eco2mix-national-cons-def/exports/json?lang=fr&timezone=UTC".into(),
            source_sha256: SAMPLE_SHA256.into(),
            license: "Etalab 2.0".into(),
            notes: vec![
                "Mix électrique national FR — agrégat annuel TWh par source.".into(),
                "Calcul : Σ(MW × 0.5h)/1e6 sur les pas 30-min réalisés.".into(),
            ],
        },
        mix: RteMixSourceTotals {
            nuclear_twh: 320.208,
            hydro_twh: 58.108,
            wind_twh: 50.866,
            solar_twh: 22.204,
            gas_twh: 29.104,
            coal_twh: 0.826,
            oil_twh: 0.952,
            bioenergies_twh: 10.318,
            pumped_twh: -5.72,
            exchange_net_twh: -50.688,
            total_production_twh: 486.866,
            records_processed: 35_040,
            year: 2023,
        },
    }
}

#[test]
fn territoire_sample_is_read_field_by_field() {
    let artifact = load_territoire_fr(&fixture(TERRITOIRE)).unwrap();

    assert_eq!(artifact, expected_territoire());
}

#[test]
fn rte_mix_sample_is_read_field_by_field() {
    let artifact = load_rte_mix(&fixture(RTE_MIX)).unwrap();

    assert_eq!(artifact, expected_rte_mix());
}

/// Adds an unknown key to every object of the sample.
fn with_unknown_fields(value: &mut Value) {
    match value {
        Value::Object(map) => {
            map.values_mut().for_each(with_unknown_fields);
            map.insert("unknown_to_geoloc".into(), Value::Bool(true));
        },
        Value::Array(items) => items.iter_mut().for_each(with_unknown_fields),
        _ => {},
    }
}

#[test]
fn fields_geoloc_does_not_know_are_ignored_in_every_struct() {
    let mut territoire: Value = serde_json::from_str(&fixture_text(TERRITOIRE)).unwrap();
    let mut rte_mix: Value = serde_json::from_str(&fixture_text(RTE_MIX)).unwrap();
    with_unknown_fields(&mut territoire);
    with_unknown_fields(&mut rte_mix);

    assert_eq!(
        serde_json::from_value::<TerritoireFrArtifact>(territoire).unwrap(),
        expected_territoire()
    );
    assert_eq!(
        serde_json::from_value::<RteMixArtifact>(rte_mix).unwrap(),
        expected_rte_mix()
    );
}

/// One variant of `text` per key occurrence, with that key renamed: `(key, variant)`.
fn renamings(text: &str) -> Vec<(String, String)> {
    text.match_indices("\":")
        .map(|(end, _)| {
            let key = &text[text[..end].rfind('"').unwrap() + 1..end];
            (
                key.to_string(),
                format!("{}_renamed{}", &text[..end], &text[end..]),
            )
        })
        .collect()
}

fn assert_every_renamed_key_is_refused<T: DeserializeOwned + Debug>(sample: &str) {
    let variants = renamings(&fixture_text(sample));
    assert!(variants.len() > 15, "{sample}: {} keys", variants.len());

    for (key, variant) in variants {
        let error = serde_json::from_str::<T>(&variant)
            .expect_err(&key)
            .to_string();

        assert!(
            error.starts_with(&format!("missing field `{key}`")),
            "{key}: {error}"
        );
    }
}

#[test]
fn a_territoire_key_renamed_on_the_ingest_side_is_refused() {
    assert_every_renamed_key_is_refused::<TerritoireFrArtifact>(TERRITOIRE);
}

#[test]
fn an_rte_mix_key_renamed_on_the_ingest_side_is_refused() {
    assert_every_renamed_key_is_refused::<RteMixArtifact>(RTE_MIX);
}

#[test]
fn the_shipped_territoire_data_is_read_and_validated() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("data/territoire_fr.json");

    let artifact = load_territoire_fr(&path).unwrap();

    assert_eq!(artifact.regions.len(), 13);
    assert_eq!(artifact.industrial_sites.len(), 200);
}
