//! The three ODRÉ fetches of `territoire_fr` against a local stub server
//! (SOB-194), through their `*_at(base_url, …)` variants. These tests pin
//! today's behavior, bugs included (see SOB-122 for the missing `annee`).

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use sobria_ingest::hash::hex_encode;
use sobria_ingest::sources::territoire_fr::{
    discover_datasets_at, fetch_industrial_sites_at, fetch_rte_mix_at,
};
use wiremock::{
    matchers::{method, path, query_param},
    Mock, MockServer, ResponseTemplate,
};

const CATALOG: &str = "/api/explore/v2.1/catalog/datasets";
const IRIS_RECORDS: &str =
    "/api/explore/v2.1/catalog/datasets/consommation-annuelle-par-iris/records";
const RTE_EXPORT: &str =
    "/api/explore/v2.1/catalog/datasets/eco2mix-national-cons-def/exports/json";

fn base(server: &MockServer) -> String {
    format!("{}{CATALOG}", server.uri())
}

fn ok_json(value: &Value) -> ResponseTemplate {
    ResponseTemplate::new(200).set_body_json(value)
}

fn sha256_of_json(value: &Value) -> String {
    hex_encode(&Sha256::digest(serde_json::to_vec(value).unwrap()))
}

async fn serve(at: &str, response: ResponseTemplate) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(at))
        .respond_with(response)
        .mount(&server)
        .await;
    server
}

/// A record `parse_industrial_record` keeps (metropolitan department, geo point, consumption).
fn site(code_iris: &str, elec: f64) -> Value {
    json!({
        "code_iris": code_iris,
        "commune": "Paris 1er",
        "code_insee_commune": "75101",
        "code_insee_departement": "75",
        "code_insee_region": "11",
        "departement": "Paris",
        "annee": 2022,
        "consommation_electricite_rte": elec,
        "consommation_gaz_grtgaz": 0.0,
        "consommation_gaz_terega": 0.0,
        "consommation_totale": elec,
        "pdl_electricite_rte": 1,
        "pdl_gaz_grtgaz": 0,
        "pdl_gaz_terega": 0,
        "pdl_total": 1,
        "geo_point_iris": {"lat": 48.86, "lon": 2.34},
    })
}

fn sites(count: usize) -> Vec<Value> {
    (0..count)
        .map(|i| site(&format!("{i:09}"), 1000.0 + i as f64))
        .collect()
}

fn err_text<T: std::fmt::Debug>(result: anyhow::Result<T>) -> String {
    format!("{:#}", result.expect_err("expected an error"))
}

// ─── discover_datasets ──────────────────────────────────────────────────────

#[tokio::test]
async fn discover_reads_each_entry_caps_the_limit_at_50_and_skips_entries_without_id() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path(CATALOG))
        .and(query_param("limit", "50"))
        .and(query_param("where", "search(\"iris\")"))
        .respond_with(ok_json(&json!({"results": [
            {"dataset_id": "a", "metas": {"default": {
                "title": "Titre A", "description": "x".repeat(200), "publisher": "RTE"}}},
            {"dataset_id": ""},
            {"dataset_id": "b", "metas": {"title": "Titre B"}},
            {"dataset_id": "c"},
        ]})))
        .expect(1)
        .mount(&server)
        .await;

    let found = discover_datasets_at(&base(&server), "iris", 80)
        .await
        .unwrap();

    assert_eq!(
        serde_json::to_value(&found).unwrap(),
        json!([
            {"dataset_id": "a", "title": "Titre A", "description": "x".repeat(180),
             "publisher": "RTE", "explore_url": "https://odre.opendatasoft.com/explore/dataset/a/"},
            {"dataset_id": "b", "title": "Titre B", "description": null,
             "publisher": null, "explore_url": "https://odre.opendatasoft.com/explore/dataset/b/"},
            {"dataset_id": "c", "title": "c", "description": null,
             "publisher": null, "explore_url": "https://odre.opendatasoft.com/explore/dataset/c/"},
        ])
    );
}

#[tokio::test]
async fn discover_returns_nothing_for_an_empty_or_missing_results_list() {
    for body in [json!({"results": []}), json!({})] {
        let server = serve(CATALOG, ok_json(&body)).await;
        let found = discover_datasets_at(&base(&server), "iris", 10)
            .await
            .unwrap();
        assert!(found.is_empty(), "{body}");
    }
}

#[tokio::test]
async fn discover_reports_the_http_status_and_the_first_500_characters_of_the_body() {
    let server = serve(
        CATALOG,
        ResponseTemplate::new(503).set_body_string("x".repeat(600)),
    )
    .await;

    let message = err_text(discover_datasets_at(&base(&server), "iris", 10).await);

    assert!(
        message.starts_with("ODRÉ catalog HTTP 503 Service Unavailable\nRéponse : "),
        "{message}"
    );
    assert_eq!(message.matches('x').count(), 500);
}

#[tokio::test]
async fn discover_fails_on_a_body_that_is_not_json() {
    let server = serve(
        CATALOG,
        ResponseTemplate::new(200).set_body_string("<html>"),
    )
    .await;

    let message = err_text(discover_datasets_at(&base(&server), "iris", 10).await);

    assert!(message.contains("expected value"), "{message}");
}

// ─── fetch_industrial_sites ─────────────────────────────────────────────────

#[tokio::test]
async fn industrial_sites_are_sorted_by_electricity_and_cut_to_the_limit() {
    let mut drom = site("971010101", 99_999.0);
    drom["code_insee_departement"] = json!("971");
    let payload = json!({"results": [
        site("751010101", 10.0), site("751010102", 30.0), site("751010103", 20.0), drom,
    ]});
    let server = serve(IRIS_RECORDS, ok_json(&payload)).await;

    let artifact = fetch_industrial_sites_at(&base(&server), 2).await.unwrap();

    let kept: Vec<f64> = artifact
        .industrial_sites
        .iter()
        .map(|s| s.consumption_mwh_elec)
        .collect();
    assert_eq!(kept, [30.0, 20.0]);
    assert_eq!(artifact.industrial_sites[0].year, 2022);
    assert_eq!(artifact.regions.len(), 13);
    assert_eq!(
        artifact.meta.source_url,
        format!("{}{IRIS_RECORDS}?limit=100&offset=0", server.uri())
    );
    assert_eq!(artifact.meta.source_sha256, sha256_of_json(&payload));
    assert!(
        artifact.meta.notes.iter().any(|n| n.contains(
            "Records bruts paginés : 4, retenus (top 2) : 2, écartés (DROM, géoloc manquante, conso ≤ 0) : 1."
        )),
        "{:?}",
        artifact.meta.notes
    );
}

#[tokio::test]
async fn industrial_pagination_asks_for_the_next_page_until_a_short_or_empty_page() {
    for second_page in [10, 0] {
        let server = MockServer::start().await;
        for (offset, limit, count) in [("0", "100", 100), ("100", "50", second_page)] {
            Mock::given(method("GET"))
                .and(path(IRIS_RECORDS))
                .and(query_param("limit", limit))
                .and(query_param("offset", offset))
                .respond_with(ok_json(&json!({"results": sites(count)})))
                .expect(1)
                .mount(&server)
                .await;
        }

        // Asks for 3 × 50 = 150 records, in pages of at most 100.
        let artifact = fetch_industrial_sites_at(&base(&server), 50).await.unwrap();

        assert_eq!(artifact.industrial_sites.len(), 50, "{second_page}");
    }
}

#[tokio::test]
async fn industrial_empty_answer_gives_no_site_and_no_error() {
    let server = serve(IRIS_RECORDS, ok_json(&json!({"results": []}))).await;

    let artifact = fetch_industrial_sites_at(&base(&server), 10).await.unwrap();

    assert!(artifact.industrial_sites.is_empty());
    assert!(artifact
        .meta
        .notes
        .iter()
        .any(|n| n.starts_with("Records bruts paginés : 0,")));
}

#[tokio::test]
async fn industrial_fails_without_a_results_array() {
    let server = serve(IRIS_RECORDS, ok_json(&json!({"total_count": 0}))).await;

    let message = err_text(fetch_industrial_sites_at(&base(&server), 10).await);

    assert_eq!(
        message,
        "payload ODRÉ : champ 'results' absent ou non-tableau"
    );
}

#[tokio::test]
async fn industrial_reports_the_http_status_with_the_page_url() {
    let server = serve(
        IRIS_RECORDS,
        ResponseTemplate::new(500).set_body_string("boom"),
    )
    .await;

    let message = err_text(fetch_industrial_sites_at(&base(&server), 10).await);

    assert_eq!(
        message,
        format!(
            "ODRÉ IRIS HTTP 500 Internal Server Error sur {}{IRIS_RECORDS}?limit=100&offset=0\nRéponse : boom",
            server.uri()
        )
    );
}

#[tokio::test]
async fn industrial_fails_on_a_body_that_is_not_json() {
    let server = serve(
        IRIS_RECORDS,
        ResponseTemplate::new(200).set_body_string("nope"),
    )
    .await;

    let message = err_text(fetch_industrial_sites_at(&base(&server), 10).await);

    assert!(
        message.starts_with("payload ODRÉ IRIS non-JSON: "),
        "{message}"
    );
}

#[tokio::test]
async fn industrial_fails_when_no_record_can_be_parsed_and_lists_the_fields() {
    let server = serve(
        IRIS_RECORDS,
        ok_json(&json!({"results": [{"foo": 1, "bar": 2}]})),
    )
    .await;

    let message = err_text(fetch_industrial_sites_at(&base(&server), 10).await);

    assert!(
        message.starts_with("0 site retenu sur 1 records. Champs ODRÉ détectés : ["),
        "{message}"
    );
    assert!(
        message.contains("foo") && message.contains("bar"),
        "{message}"
    );
}

#[tokio::test]
async fn industrial_site_year_is_zero_when_annee_is_missing_sob_122() {
    let mut record = site("751010101", 10.0);
    record.as_object_mut().unwrap().remove("annee");
    let server = serve(IRIS_RECORDS, ok_json(&json!({"results": [record]}))).await;

    let artifact = fetch_industrial_sites_at(&base(&server), 10).await.unwrap();

    assert_eq!(artifact.industrial_sites[0].year, 0);
}

// ─── fetch_rte_mix ──────────────────────────────────────────────────────────

#[tokio::test]
async fn rte_mix_sums_half_hour_steps_into_twh() {
    let server = MockServer::start().await;
    let payload = json!([
        {"nucleaire": 40000.0, "hydraulique": 8000.0, "eolien": 2000.0, "solaire": 1200.0,
         "gaz": 3000.0, "charbon": 0.0, "fioul": 100.0, "bioenergies": 900.0,
         "pompage": -1200.0, "ech_physiques": -6000.0},
        {"nucleaire": 40000.0, "hydraulique": 8000.0},
    ]);
    Mock::given(method("GET"))
        .and(path(RTE_EXPORT))
        .and(query_param("lang", "fr"))
        .and(query_param("timezone", "UTC"))
        .and(query_param(
            "where",
            "date_heure>=date'2023-01-01' AND date_heure<date'2024-01-01'",
        ))
        .and(query_param(
            "select",
            "nucleaire,hydraulique,eolien,solaire,gaz,charbon,fioul,bioenergies,pompage,ech_physiques",
        ))
        .respond_with(ok_json(&payload))
        .expect(1)
        .mount(&server)
        .await;

    let artifact = fetch_rte_mix_at(&base(&server), 2023).await.unwrap();

    // MW × 0.5 h / 10⁶ per record, rounded to 3 decimals.
    let mix = &artifact.mix;
    assert_eq!(mix.nuclear_twh, 0.04);
    assert_eq!(mix.hydro_twh, 0.008);
    assert_eq!(mix.wind_twh, 0.001);
    assert_eq!(mix.solar_twh, 0.001);
    assert_eq!(mix.oil_twh, 0.0, "0.00005 TWh rounds to 0");
    assert_eq!(mix.pumped_twh, -0.001);
    assert_eq!(mix.exchange_net_twh, -0.003);
    // 0.04 + 0.008 + 0.001 + 0.0006 + 0.0015 + 0.00005 + 0.00045 - 0.0006, before rounding.
    assert_eq!(
        mix.total_production_twh, 0.051,
        "exchanges are not production"
    );
    assert_eq!(mix.records_processed, 2);
    assert_eq!(mix.year, 2023);
    assert_eq!(artifact.meta.source_sha256, sha256_of_json(&payload));
    assert!(artifact.meta.source_url.starts_with(&format!(
        "{}{RTE_EXPORT}?lang=fr&timezone=UTC&where=",
        server.uri()
    )));
}

#[tokio::test]
async fn rte_mix_of_an_empty_export_is_all_zeros() {
    let server = serve(RTE_EXPORT, ok_json(&json!([]))).await;

    let mix = fetch_rte_mix_at(&base(&server), 2023).await.unwrap().mix;

    assert_eq!(mix.total_production_twh, 0.0);
    assert_eq!(mix.records_processed, 0);
}

#[tokio::test]
async fn rte_mix_ignores_values_that_are_not_numbers() {
    let server = serve(
        RTE_EXPORT,
        ok_json(&json!([{"nucleaire": "n/a", "hydraulique": 8000.0}])),
    )
    .await;

    let mix = fetch_rte_mix_at(&base(&server), 2023).await.unwrap().mix;

    assert_eq!(mix.nuclear_twh, 0.0);
    assert_eq!(mix.hydro_twh, 0.004);
    assert_eq!(mix.records_processed, 1);
}

#[tokio::test]
async fn rte_mix_reports_the_http_status_with_the_url() {
    let server = serve(
        RTE_EXPORT,
        ResponseTemplate::new(404).set_body_string("missing"),
    )
    .await;

    let message = err_text(fetch_rte_mix_at(&base(&server), 2023).await);

    assert!(
        message.starts_with(&format!(
            "RTE eco2mix HTTP 404 Not Found sur {}{RTE_EXPORT}?",
            server.uri()
        )),
        "{message}"
    );
    assert!(message.ends_with("\nRéponse : missing"), "{message}");
}

#[tokio::test]
async fn rte_mix_fails_on_a_body_that_is_not_a_json_array() {
    for body in ["{\"results\": []}", "garbage"] {
        let server = serve(RTE_EXPORT, ResponseTemplate::new(200).set_body_string(body)).await;

        let message = err_text(fetch_rte_mix_at(&base(&server), 2023).await);

        assert!(
            message.starts_with("payload eco2mix non-JSON tableau: "),
            "{body}: {message}"
        );
    }
}

#[tokio::test]
async fn a_trailing_slash_on_the_base_url_is_ignored() {
    let server = MockServer::start().await;
    for (at, body) in [
        (CATALOG, json!({})),
        (IRIS_RECORDS, json!({"results": []})),
        (RTE_EXPORT, json!([])),
    ] {
        Mock::given(method("GET"))
            .and(path(at))
            .respond_with(ok_json(&body))
            .mount(&server)
            .await;
    }
    let slashed = format!("{}/", base(&server));

    discover_datasets_at(&slashed, "iris", 10).await.unwrap();
    fetch_industrial_sites_at(&slashed, 10).await.unwrap();
    fetch_rte_mix_at(&slashed, 2023).await.unwrap();
}
