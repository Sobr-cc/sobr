//! Pins how `sobria-geoloc` reads the JSON files written by `sobria-ingest` (SOB-189).
//!
//! The samples in `tests/fixtures` have the shape of the ingest output. The test
//! `territoire_contract.rs` of `sobria-ingest` checks that the ingest structs write
//! exactly these files, so both sides are tied to the same samples.

use std::path::{Path, PathBuf};

use sobria_geoloc::{
    load_rte_mix, load_territoire_fr, ArtifactMeta, IndustrialSite, RegionMeta, RteMixArtifact,
    RteMixMeta, RteMixSourceTotals, TerritoireFrArtifact,
};

const TERRITOIRE: &str = "territoire_fr.sample.json";
const RTE_MIX: &str = "rte_mix_fr.sample.json";

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
            fetched_at: "2026-05-13T13:54:58.621908300+00:00".into(),
            source_url: "https://odre.opendatasoft.com/api/explore/v2.1/catalog/datasets/consommation-annuelle-par-iris/records?limit=100&offset=0".into(),
            source_sha256: "66b2d2558e5b002fa4ce36357b6d2e5d4d237244fd1372a6b592c85eff915aeb".into(),
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
            fetched_at: "2026-05-13T13:55:07.230448800+00:00".into(),
            source_url: "https://odre.opendatasoft.com/api/explore/v2.1/catalog/datasets/eco2mix-national-cons-def/exports/json?lang=fr&timezone=UTC".into(),
            source_sha256: "a2250fe722dd39b0399bb2c66e638a178ebe324901d42fc9bb79ceb4bd0b0637".into(),
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

#[test]
fn fields_geoloc_does_not_know_are_ignored() {
    let territoire = fixture_text(TERRITOIRE)
        .replace(
            "\"license\": \"Etalab 2.0\",",
            "\"license\": \"Etalab 2.0\",\n    \"recalibrated_at\": \"2026-05-13T18:00:00+00:00\",",
        )
        .replace(
            "\"regions\": [",
            "\"generated_by\": \"sobria-ingest\",\n  \"regions\": [",
        );
    let rte_mix = fixture_text(RTE_MIX).replace(
        "\"license\": \"Etalab 2.0\",",
        "\"license\": \"Etalab 2.0\",\n    \"recalibrated_at\": \"2026-05-13T18:00:00+00:00\",",
    );

    assert_eq!(
        serde_json::from_str::<TerritoireFrArtifact>(&territoire).unwrap(),
        expected_territoire()
    );
    assert_eq!(
        serde_json::from_str::<RteMixArtifact>(&rte_mix).unwrap(),
        expected_rte_mix()
    );
}

const TERRITOIRE_FIELDS: &[&str] = &[
    "_meta",
    "regions",
    "industrial_sites",
    "version",
    "fetched_at",
    "source_url",
    "source_sha256",
    "license",
    "notes",
    "region_iso",
    "name",
    "insee_code",
    "centroid_lat",
    "centroid_lon",
    "nuclear_share_pct",
    "code_iris",
    "commune",
    "commune_code",
    "department_code",
    "department_label",
    "region_insee_code",
    "lat",
    "lon",
    "consumption_mwh_elec",
    "consumption_mwh_gas_grtgaz",
    "consumption_mwh_gas_terega",
    "consumption_total_mwh",
    "pdl_count_elec",
    "pdl_count_gas",
    "pdl_total",
    "year",
];

const RTE_MIX_FIELDS: &[&str] = &[
    "_meta",
    "mix",
    "version",
    "fetched_at",
    "source_url",
    "source_sha256",
    "license",
    "notes",
    "nuclear_twh",
    "hydro_twh",
    "wind_twh",
    "solar_twh",
    "gas_twh",
    "coal_twh",
    "oil_twh",
    "bioenergies_twh",
    "pumped_twh",
    "exchange_net_twh",
    "total_production_twh",
    "records_processed",
    "year",
];

fn renamed(text: &str, field: &str) -> String {
    let key = format!("\"{field}\":");
    assert!(text.contains(&key), "{field} is not in the sample");
    text.replace(&key, &format!("\"{field}_renamed\":"))
}

#[test]
fn a_territoire_field_renamed_on_the_ingest_side_is_refused() {
    let text = fixture_text(TERRITOIRE);

    for field in TERRITOIRE_FIELDS {
        let error = serde_json::from_str::<TerritoireFrArtifact>(&renamed(&text, field))
            .expect_err(field)
            .to_string();

        assert!(
            error.starts_with(&format!("missing field `{field}`")),
            "{field}: {error}"
        );
    }
}

#[test]
fn an_rte_mix_field_renamed_on_the_ingest_side_is_refused() {
    let text = fixture_text(RTE_MIX);

    for field in RTE_MIX_FIELDS {
        let error = serde_json::from_str::<RteMixArtifact>(&renamed(&text, field))
            .expect_err(field)
            .to_string();

        assert!(
            error.starts_with(&format!("missing field `{field}`")),
            "{field}: {error}"
        );
    }
}
