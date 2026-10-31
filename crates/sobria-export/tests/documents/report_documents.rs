//! The CSRD report: the text of its PDF, read back from the file, and its
//! PROV-O document.

use sobria_audit::AuditEntry;
use sobria_core::EmpreinteMethod::{self, AfnorSobria, EcoLogits};
use sobria_export::{
    build_datasheet, generate_report, report::AFNOR_HEADER_MARKER, DatasheetOptions,
};

use crate::common::{
    at, entry, golden::assert_golden, mask, node, pdf_pages, pdf_text, project, report,
    report_request, report_text, sha256_hex,
};
use serde_json::json;

fn ecologits_only() -> Vec<AuditEntry> {
    vec![
        entry(1, EcoLogits, "gpt-4o-mini", 1.0),
        entry(2, EcoLogits, "mistral-large", 2.0),
    ]
}

fn two_methods() -> Vec<AuditEntry> {
    vec![
        entry(1, AfnorSobria, "gpt-4o-mini", 1.0),
        entry(2, EcoLogits, "mistral-large", 2.0),
    ]
}

#[test]
fn text_of_the_report_for_an_ecologits_only_ledger_is_pinned() {
    assert_golden(
        "report_text_ecologits_only",
        &report_text(&ecologits_only()),
    );
}

#[test]
fn text_of_the_report_for_a_ledger_with_two_methods_is_pinned() {
    assert_golden("report_text_two_methods", &report_text(&two_methods()));
}

/// The lines from "Distribution Monte-Carlo" to the last source.
fn methodology_block(text: &[String]) -> &[String] {
    let from = text
        .iter()
        .position(|l| l.starts_with("Distribution Monte-Carlo"))
        .expect("the block starts");
    let to = text
        .iter()
        .position(|l| l.starts_with("  - Mytton"))
        .expect("the block ends");
    &text[from..=to]
}

#[test]
fn report_quirk_the_afnor_header_and_the_methodology_block_are_fixed_text() {
    let ecologits = report_text(&[entry(1, EcoLogits, "gpt-4o-mini", 1.0)]);
    let afnor = report_text(&[entry(1, AfnorSobria, "gpt-4o-mini", 1.0)]);

    // The ledger only used EcoLogits and the report still claims AFNOR SPEC 2314.
    assert!(ecologits.iter().any(|l| l == AFNOR_HEADER_MARKER));
    assert!(afnor.iter().any(|l| l == AFNOR_HEADER_MARKER));
    // The methodology block (Monte-Carlo, seed, EcoLogits port, sources) does not
    // depend on the method either.
    assert_eq!(methodology_block(&ecologits), methodology_block(&afnor));
    assert_eq!(methodology_block(&ecologits).len(), 10);
}

#[test]
fn report_quirk_a_character_outside_winansi_is_printed_as_a_question_mark() {
    // The built-in fonts only cover WinAnsi (SOB-241). The PROV-O document keeps
    // the name as typed, so the report contradicts its own provenance.
    let mut req = report_request("fr");
    req.organization_name = "Łódź Analytics".into();
    let art = generate_report(&req, &ecologits_only()).expect("report");

    let text = pdf_text(&art.pdf_bytes);
    assert!(
        text.contains(&"Organisation : ?ód? Analytics".to_string()),
        "{text:?}"
    );
    assert_eq!(
        art.provo_jsonld["@graph"][0]["sobria:organizationName"],
        "Łódź Analytics"
    );
}

#[test]
fn report_quirk_the_locale_is_ignored() {
    let entries = ecologits_only();
    let (fr, fr_again, en) = (
        report("fr", &entries),
        report("fr", &entries),
        report("en", &entries),
    );

    // The pages carry no date and no document id (the file does): two runs on the
    // same input give the same pages, and so do "fr" and "en".
    assert_eq!(pdf_pages(&fr.pdf_bytes), pdf_pages(&fr_again.pdf_bytes));
    assert_eq!(pdf_pages(&fr.pdf_bytes), pdf_pages(&en.pdf_bytes));
}

#[test]
fn report_quirk_issued_on_and_the_prov_times_are_the_period_end_even_in_the_future() {
    let mut req = report_request("fr");
    req.period_end = at(2099, 12, 31, 12, 30);
    let art = generate_report(&req, &ecologits_only()).expect("report");

    let text = pdf_text(&art.pdf_bytes);
    assert!(
        text.iter().any(|l| l == "Émis le 2099-12-31 12:30 UTC"),
        "{text:?}"
    );
    let report_id = "sobria:report-20260101-to-20991231";
    let report_node = node(&art.provo_jsonld, report_id);
    let activity = node(&art.provo_jsonld, &format!("{report_id}-activity"));
    let period_end = "2099-12-31T12:30:00+00:00";
    assert_eq!(report_node["prov:generatedAtTime"]["@value"], period_end);
    assert_eq!(report_node["schema:datePublished"], "2099-12-31");
    assert_eq!(activity["prov:startedAtTime"]["@value"], period_end);
    assert_eq!(activity["prov:endedAtTime"]["@value"], period_end);
}

#[test]
fn prov_o_document_of_a_report_with_two_methods_is_pinned() {
    let art = report("fr", &two_methods());
    let mut prov = art.provo_jsonld.clone();

    // The PDF carries dates and a random document id, so its hash changes on
    // every run: check it here, then mask it.
    assert_eq!(art.pdf_sha256, sha256_hex(&art.pdf_bytes));
    assert_eq!(
        prov["@graph"][0]["schema:contentSha256"],
        art.pdf_sha256.as_str()
    );
    mask(&mut prov, "/@graph/0/schema:contentSha256", "<sha256>");
    // The agent node states the version of this crate.
    assert_eq!(
        node(&prov, "sobria:agent-sobria-export")["schema:version"],
        env!("CARGO_PKG_VERSION")
    );
    mask(&mut prov, "/@graph/4/schema:version", "<crate-version>");

    assert_golden("prov_o_two_methods", &prov);
}

/// `(method, label in the PDF, label in the datasheet, label in PROV-O, legacy
/// PROV-O label)`: the same two methods are named in four places of the crate.
const LABELS: &[(EmpreinteMethod, &str, &str, &str, &str)] = &[
    (
        AfnorSobria,
        "AFNOR SPEC 2314 (Sobr.ia) - referentiel francais",
        "AFNOR SPEC 2314 (Sobr.ia) — référentiel français, formule linéaire-par-token + Monte-Carlo N=10⁴",
        "AFNOR SPEC 2314 (Sobr.ia) — Monte-Carlo distributional",
        "AFNOR SPEC 2314 (Sobr.ia)",
    ),
    (
        EcoLogits,
        "EcoLogits 2026-01 (port direct, doi:10.21105/joss.07471, CC BY-SA 4.0)",
        "EcoLogits 2026-01 — port direct des formules officielles (doi:10.21105/joss.07471, CC BY-SA 4.0)",
        "EcoLogits 2026-01 — port direct, doi:10.21105/joss.07471",
        "EcoLogits 2026-01",
    ),
];

#[test]
fn each_method_is_labelled_as_pinned_in_the_report_the_datasheet_and_prov_o() {
    for &(method, pdf, datasheet, prov_o, legacy) in LABELS {
        let entries = [entry(1, method, "gpt-4o-mini", 1.0)];

        let art = report("fr", &entries);
        let line = format!("Methodologie utilisee : {pdf}");
        assert!(
            pdf_text(&art.pdf_bytes).contains(&line),
            "{method:?}: {line}"
        );

        // `app_version` of the request is "0.2.0".
        let engine = node(&art.provo_jsonld, "sobria:estimator-engine-v0.2.0");
        assert_eq!(engine["sobria:methodologiesUsed"], json!([prov_o]));
        assert_eq!(engine["sobria:methodology"], legacy);

        let sheet = build_datasheet(&project(), &entries, &DatasheetOptions::default());
        let datasheet_node = node(&sheet.jsonld, "sobria:project-1/datasheet");
        assert_eq!(
            datasheet_node["sobria:methodologiesUsed"],
            json!([datasheet])
        );
        let process = datasheet_node["sobria:collectionProcess"]
            .as_str()
            .expect("text");
        assert!(process.contains(datasheet), "{method:?}: {process}");
    }
}
