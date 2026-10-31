//! Characterization tests of the CSRD report and its PROV-O document (SOB-221).
//!
//! They record what the documents say today, not that they are well formed
//! (the unit tests do that): the text of the PDF, read back from its bytes, and
//! the PROV-O graph. Tests named `quirk` pin behavior the codebase audit
//! flagged; SOB-23 (cleanup) and the Triage issues listed in the PR decide
//! what to do with it.
//!
//! To rewrite the golden files, see `test-support/golden.rs`
//! (`SOBRIA_UPDATE_GOLDEN=1`).

mod common;

use common::{
    at, entry, golden::assert_golden, node, pdf_text, project, report_request, without_document_id,
    AFNOR_HEADER,
};
use serde_json::json;
use sobria_core::EmpreinteMethod::{AfnorSobria, EcoLogits};
use sobria_export::{build_datasheet, generate_report, DatasheetOptions};

fn ecologits_only() -> Vec<sobria_audit::AuditEntry> {
    vec![
        entry(1, EcoLogits, "gpt-4o-mini", 1.0),
        entry(2, EcoLogits, "mistral-large", 2.0),
    ]
}

fn two_methods() -> Vec<sobria_audit::AuditEntry> {
    vec![
        entry(1, AfnorSobria, "gpt-4o-mini", 1.0),
        entry(2, EcoLogits, "mistral-large", 2.0),
    ]
}

#[test]
fn text_of_the_report_for_an_ecologits_only_ledger_is_pinned() {
    let art = generate_report(&report_request("fr"), &ecologits_only()).expect("report");
    assert_golden("report_text_ecologits_only", &pdf_text(&art.pdf_bytes));
}

#[test]
fn text_of_the_report_for_a_ledger_with_two_methods_is_pinned() {
    let art = generate_report(&report_request("fr"), &two_methods()).expect("report");
    assert_golden("report_text_two_methods", &pdf_text(&art.pdf_bytes));
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
    let report_text = |entries: &[sobria_audit::AuditEntry]| {
        pdf_text(
            &generate_report(&report_request("fr"), entries)
                .expect("report")
                .pdf_bytes,
        )
    };
    let ecologits = report_text(&[entry(1, EcoLogits, "gpt-4o-mini", 1.0)]);
    let afnor = report_text(&[entry(1, AfnorSobria, "gpt-4o-mini", 1.0)]);

    // The ledger only used EcoLogits and the report still claims AFNOR SPEC 2314.
    assert!(ecologits.iter().any(|l| l == AFNOR_HEADER));
    assert!(afnor.iter().any(|l| l == AFNOR_HEADER));
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
    let report = |locale: &str| generate_report(&report_request(locale), &entries).expect("report");
    let (fr, fr_again, en) = (report("fr"), report("fr"), report("en"));

    // Two runs on the same input differ by the random document id and nothing else.
    assert_eq!(
        without_document_id(&fr.pdf_bytes),
        without_document_id(&fr_again.pdf_bytes)
    );
    // The English report is the French one.
    assert_eq!(
        without_document_id(&fr.pdf_bytes),
        without_document_id(&en.pdf_bytes)
    );
}

#[test]
fn report_quirk_issued_on_and_the_prov_times_are_the_period_end_even_in_the_future() {
    let mut req = report_request("fr");
    req.period_end = at(2099, 12, 31, 12, 30);
    let art = generate_report(&req, &ecologits_only()).expect("report");

    assert!(
        pdf_text(&art.pdf_bytes)
            .iter()
            .any(|l| l == "Émis le 2099-12-31 12:30 UTC"),
        "{:?}",
        pdf_text(&art.pdf_bytes)
    );
    let prov = &art.provo_jsonld;
    let report = node(prov, "sobria:report-");
    let activity = node(prov, "sobria:report-20260101-to-20991231-activity");
    let period_end = "2099-12-31T12:30:00+00:00";
    assert_eq!(report["prov:generatedAtTime"]["@value"], period_end);
    assert_eq!(report["schema:datePublished"], "2099-12-31");
    assert_eq!(activity["prov:startedAtTime"]["@value"], period_end);
    assert_eq!(activity["prov:endedAtTime"]["@value"], period_end);
}

#[test]
fn prov_o_document_of_a_report_with_two_methods_is_pinned() {
    let art = generate_report(&report_request("fr"), &two_methods()).expect("report");
    let mut prov = art.provo_jsonld.clone();
    // The PDF carries a random document id, so its hash changes on every run.
    assert_eq!(
        prov["@graph"][0]["schema:contentSha256"],
        art.pdf_sha256.as_str()
    );
    prov["@graph"][0]["schema:contentSha256"] = json!("<sha256>");
    assert_golden("prov_o_two_methods", &prov);
}

/// `(method, label in the PDF, label in the datasheet, label in PROV-O, legacy
/// PROV-O label)`: the same two methods are named in four places of the crate.
const LABELS: [(sobria_core::EmpreinteMethod, &str, &str, &str, &str); 2] = [
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
    for (method, pdf, datasheet, prov_o, legacy) in LABELS {
        let entries = [entry(1, method, "gpt-4o-mini", 1.0)];

        let art = generate_report(&report_request("fr"), &entries).expect("report");
        let line = format!("Methodologie utilisee : {pdf}");
        assert!(
            pdf_text(&art.pdf_bytes).contains(&line),
            "{method:?}: {line}"
        );

        let engine = node(&art.provo_jsonld, "sobria:estimator-engine-v");
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
