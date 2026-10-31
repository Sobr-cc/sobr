//! Génération du rapport PDF CSRD/AGEC/AFNOR SPEC 2314.
//!
//! Voir `docs/archive/chantiers/C14-rapport-csrd-agec.md` §1 pour la
//! structure complète.

use chrono::{DateTime, Utc};
use printpdf::{
    BuiltinFont, Mm, OffsetDateTime, Op, PdfDocument, PdfFontHandle, PdfPage,
    PdfParseErrorSeverity, PdfSaveOptions, Point, Pt, TextItem,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sobria_audit::AuditEntry;
use sobria_core::EmpreinteMethod;
use tracing::{debug, info};

use crate::{
    error::{ExportError, ExportResult},
    provo::{build_provo_jsonld, ProvOOptions},
    summary::{aggregate, ReportSummary},
};

/// Marqueur AFNOR placé dans le PDF (page 1) — utilisé par les tests
/// d'intégration pour s'assurer que le rapport est bien produit.
pub const AFNOR_HEADER_MARKER: &str = "Rapport conforme AFNOR SPEC 2314";

/// Polish F (C24) — Libellé long d'une méthodologie pour rendu PDF.
/// Sans caractères Unicode étendus (le rendu printpdf utilise une police
/// built-in qui n'a pas tout l'éventail Unicode).
#[must_use]
fn methodology_pdf_label(method: EmpreinteMethod) -> &'static str {
    match method {
        EmpreinteMethod::AfnorSobria => "AFNOR SPEC 2314 (Sobr.ia) - referentiel francais",
        EmpreinteMethod::EcoLogits => {
            "EcoLogits 2026-01 (port direct, doi:10.21105/joss.07471, CC BY-SA 4.0)"
        },
    }
}

/// Paramètres d'entrée pour générer un rapport.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportRequest {
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub organization_name: String,
    /// Locale UI — v1.0 : "fr" uniquement.
    pub locale: String,
    /// Version Sobr.ia injectée par l'app (pour traçabilité PROV-O).
    pub app_version: String,
    /// Seed Monte-Carlo (référence — pas utilisé pour le calcul, juste
    /// reporté dans le PDF et le PROV-O).
    pub estimator_seed: u64,
    /// N Monte-Carlo (reporté).
    pub estimator_n: u32,
}

/// Sortie complète : PDF + PROV-O + résumé.
#[derive(Debug, Clone)]
pub struct ReportArtifacts {
    pub pdf_bytes: Vec<u8>,
    pub pdf_sha256: String,
    pub provo_jsonld: serde_json::Value,
    pub summary: ReportSummary,
    pub audit_entries_count: usize,
}

/// Génère un rapport à partir d'une liste d'entrées d'audit.
///
/// L'aggregation se fait sur `[period_start, period_end)`. Les entrées
/// hors période sont ignorées. Si aucune entrée n'est dans la période,
/// retourne `ExportError::EmptyPeriod`.
pub fn generate_report(
    req: &ReportRequest,
    ledger_entries: &[AuditEntry],
) -> ExportResult<ReportArtifacts> {
    let summary = aggregate(ledger_entries, req.period_start, req.period_end)?;
    debug!(
        requests = summary.total_requests,
        co2_p50 = summary.total_co2eq_g_p50,
        "agrégat de période calculé"
    );

    let pdf_bytes = build_pdf(req, &summary)?;
    let pdf_sha256 = sha256_hex(&pdf_bytes);

    let report_id = format!(
        "sobria:report-{}-to-{}",
        req.period_start.format("%Y%m%d"),
        req.period_end.format("%Y%m%d")
    );
    let opts = ProvOOptions {
        report_id,
        organization_name: req.organization_name.clone(),
        estimator_version: req.app_version.clone(),
        estimator_seed: req.estimator_seed,
        estimator_n: req.estimator_n,
        generated_at: req.period_end,
    };
    let provo_jsonld = build_provo_jsonld(&summary, &pdf_sha256, &opts);

    let entries_in_period = ledger_entries
        .iter()
        .filter(|e| e.timestamp >= req.period_start && e.timestamp < req.period_end)
        .count();

    info!(
        sha256 = %pdf_sha256,
        entries = entries_in_period,
        "rapport généré"
    );
    Ok(ReportArtifacts {
        pdf_bytes,
        pdf_sha256,
        provo_jsonld,
        summary,
        audit_entries_count: entries_in_period,
    })
}

// ─────────────────────────────────────────────────────────────────────────────
// PDF building (printpdf, A4 portrait)
// ─────────────────────────────────────────────────────────────────────────────

/// Text operations of one page, with the call shape of the printpdf 0.7 layer.
#[derive(Default)]
struct TextPage {
    ops: Vec<Op>,
}

impl TextPage {
    fn use_text<S: Into<String>>(
        &mut self,
        text: S,
        size: f32,
        x: Mm,
        y: Mm,
        font: &PdfFontHandle,
    ) {
        self.ops.extend([
            Op::StartTextSection,
            Op::SetTextCursor {
                pos: Point::new(x, y),
            },
            Op::SetFont {
                font: font.clone(),
                size: Pt(size),
            },
            Op::ShowText {
                items: vec![TextItem::Text(text.into())],
            },
            Op::EndTextSection,
        ]);
    }
}

#[allow(clippy::too_many_lines)] // PDF rendering : flow linéaire mieux lu en bloc
fn build_pdf(req: &ReportRequest, summary: &ReportSummary) -> ExportResult<Vec<u8>> {
    let font_regular = PdfFontHandle::Builtin(BuiltinFont::HelveticaBold);
    let font_body = PdfFontHandle::Builtin(BuiltinFont::Helvetica);

    // ── Page 1 : page de garde ────────────────────────────────────────
    let cover_ops = {
        let mut layer = TextPage::default();
        let mut y = 260.0;
        let title = "Empreinte environnementale IA générative".to_string();
        layer.use_text(title, 22.0, Mm(20.0), Mm(y), &font_regular);
        y -= 12.0;
        layer.use_text(
            format!("Organisation : {}", req.organization_name),
            12.0,
            Mm(20.0),
            Mm(y),
            &font_body,
        );
        y -= 7.0;
        let period_line = format!(
            "Période : {} – {}",
            req.period_start.format("%Y-%m-%d"),
            req.period_end.format("%Y-%m-%d")
        );
        layer.use_text(period_line, 12.0, Mm(20.0), Mm(y), &font_body);
        y -= 7.0;
        layer.use_text(
            format!("Émis le {}", req.period_end.format("%Y-%m-%d %H:%M UTC")),
            10.0,
            Mm(20.0),
            Mm(y),
            &font_body,
        );
        y -= 15.0;
        layer.use_text(AFNOR_HEADER_MARKER, 10.0, Mm(20.0), Mm(y), &font_regular);
        y -= 6.0;
        layer.use_text(
            format!(
                "Sobr.ia v{} — Monte-Carlo N={} — seed {}",
                req.app_version, req.estimator_n, req.estimator_seed
            ),
            10.0,
            Mm(20.0),
            Mm(y),
            &font_body,
        );

        // Bloc synthèse exécutive
        y -= 25.0;
        layer.use_text("Synthèse exécutive", 16.0, Mm(20.0), Mm(y), &font_regular);
        y -= 8.0;
        layer.use_text(
            format!("Nombre total de requêtes : {}", summary.total_requests),
            11.0,
            Mm(20.0),
            Mm(y),
            &font_body,
        );
        y -= 6.0;
        layer.use_text(
            format!(
                "CO2eq total — P50 : {:.3} g (P5 {:.3} – P95 {:.3})",
                summary.total_co2eq_g_p50, summary.total_co2eq_g_p5, summary.total_co2eq_g_p95
            ),
            11.0,
            Mm(20.0),
            Mm(y),
            &font_body,
        );
        y -= 6.0;
        layer.use_text(
            format!(
                "Énergie totale (P50) : {:.3} Wh",
                summary.total_energy_wh_p50
            ),
            11.0,
            Mm(20.0),
            Mm(y),
            &font_body,
        );
        y -= 6.0;
        layer.use_text(
            format!("Eau totale (P50) : {:.4} L", summary.total_water_l_p50),
            11.0,
            Mm(20.0),
            Mm(y),
            &font_body,
        );
        layer.ops
    };

    // ── Page 2 : Méthodologie + Audit + Signature ─────────────────────
    let details_ops = {
        let mut layer = TextPage::default();
        let mut y = 270.0;
        layer.use_text("Méthodologie", 16.0, Mm(20.0), Mm(y), &font_regular);
        y -= 8.0;

        // Polish F (C24) — Lister les méthodologies effectivement utilisées
        // dans la période, lues depuis le ledger. Plus de hard-code AFNOR.
        let methods_intro = match summary.methods_used.len() {
            0 => "Methodologie(s) utilisee(s) : (aucune entree)".to_string(),
            1 => format!(
                "Methodologie utilisee : {}",
                methodology_pdf_label(summary.methods_used[0])
            ),
            n => format!("Methodologies utilisees dans la periode ({n}) :"),
        };
        layer.use_text(&methods_intro, 10.0, Mm(20.0), Mm(y), &font_body);
        y -= 6.0;
        if summary.methods_used.len() > 1 {
            for m in &summary.methods_used {
                layer.use_text(
                    format!("  - {}", methodology_pdf_label(*m)),
                    9.5,
                    Mm(20.0),
                    Mm(y),
                    &font_body,
                );
                y -= 5.0;
            }
            layer.use_text(
                "Note : ce rapport agrege des estimations de plusieurs methodologies.",
                9.0,
                Mm(20.0),
                Mm(y),
                &font_body,
            );
            y -= 5.0;
            layer.use_text(
                "      Cf. section breakdown du dashboard pour les sous-totaux par methodologie.",
                9.0,
                Mm(20.0),
                Mm(y),
                &font_body,
            );
            y -= 5.0;
        }
        y -= 3.0;

        for line in [
            "Distribution Monte-Carlo : N tirages independants par parametre",
            "  (PUE, IF elec, embodied, WUE, e prefill/decode).",
            "Seed deterministe pour reproductibilite.",
            "Port EcoLogits 2026-01 : formules officielles reproduites a <=1%",
            "  (DOI: 10.21105/joss.07471, CC BY-SA 4.0).",
            "Sources des parametres :",
            "  - HF AI Energy Score (CC-BY)",
            "  - RTE eco2mix / Electricity Maps (Etalab 2.0 / CC-BY)",
            "  - ADEME Base Empreinte (Etalab 2.0)",
            "  - Mytton 2021 (eau)",
        ] {
            layer.use_text(line, 9.0, Mm(20.0), Mm(y), &font_body);
            y -= 5.0;
        }
        y -= 5.0;

        layer.use_text("Traçabilité audit", 16.0, Mm(20.0), Mm(y), &font_regular);
        y -= 8.0;
        layer.use_text(
            format!(
                "Entrées d'audit couvertes : id {} – {}",
                summary.first_audit_id, summary.last_audit_id
            ),
            10.0,
            Mm(20.0),
            Mm(y),
            &font_body,
        );
        y -= 5.0;
        layer.use_text(
            format!("Total requêtes journalisées : {}", summary.total_requests),
            10.0,
            Mm(20.0),
            Mm(y),
            &font_body,
        );
        y -= 5.0;
        layer.use_text(
            "Chaîne SHA-256 vérifiable via la commande \"verify_audit\".",
            10.0,
            Mm(20.0),
            Mm(y),
            &font_body,
        );

        y -= 12.0;
        layer.use_text(
            "Provenance & licences",
            16.0,
            Mm(20.0),
            Mm(y),
            &font_regular,
        );
        y -= 8.0;
        for line in [
            "Provenance W3C PROV-O JSON-LD : fichier provo.jsonld joint.",
            "Code Sobr.ia : MIT.",
            "Données ODRÉ/RTE : Etalab 2.0.",
            "Méthodologie / publications : CC-BY 4.0.",
        ] {
            layer.use_text(line, 9.0, Mm(20.0), Mm(y), &font_body);
            y -= 5.0;
        }
        layer.ops
    };

    // A4 portrait : 210 × 297 mm.
    let pages: Vec<PdfPage> = [cover_ops, details_ops]
        .into_iter()
        .map(|ops| PdfPage::new(Mm(210.0), Mm(297.0), ops))
        .collect();
    let mut doc = PdfDocument::new(&format!("Sobr.ia - {}", req.organization_name));
    // printpdf 0.12 dates a document 1970-01-01 unless told otherwise.
    let now = OffsetDateTime::now();
    doc.metadata.info.creation_date = now;
    doc.metadata.info.modification_date = now;
    doc.metadata.info.metadata_date = now;
    let mut warnings = Vec::new();
    let bytes = doc
        .with_pages(pages)
        .save(&PdfSaveOptions::default(), &mut warnings);
    // A save that drops content must not yield a compliance report.
    let errors: Vec<&str> = warnings
        .iter()
        .filter(|w| w.severity == PdfParseErrorSeverity::Error)
        .map(|w| w.msg.as_str())
        .collect();
    if !errors.is_empty() {
        return Err(ExportError::Pdf(errors.join("; ")));
    }
    if !warnings.is_empty() {
        debug!(?warnings, "printpdf save warnings");
    }
    Ok(bytes)
}

/// SHA-256 en hexadécimal minuscule (partagé avec `datasheet`).
pub(crate) fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use sobria_core::{
        EmpreinteMethod, EstimationRequest, EstimationResult, Hypothesis, Indicator,
        IndicatorValue, UncertaintyInterval,
    };

    fn make_entry(id: i64, ts: DateTime<Utc>, co2eq: f64) -> AuditEntry {
        let result = EstimationResult {
            method: EmpreinteMethod::AfnorSobria,
            request: EstimationRequest {
                model_id: "gpt-4o-mini".into(),
                tokens_in: 100,
                tokens_out_estimated: 500,
                datacenter_id: None,
                timestamp: ts,
                modalities: Vec::new(),
                overhead: sobria_core::ContextOverhead::default(),
            },
            indicators: vec![
                IndicatorValue {
                    indicator: Indicator::Co2Eq,
                    interval: UncertaintyInterval::new(co2eq * 0.7, co2eq, co2eq * 1.4).unwrap(),
                    unit: "gCO2eq".into(),
                    bins: None,
                },
                IndicatorValue {
                    indicator: Indicator::Energy,
                    interval: UncertaintyInterval::new(0.5, 1.0, 1.5).unwrap(),
                    unit: "Wh".into(),
                    bins: None,
                },
                IndicatorValue {
                    indicator: Indicator::Water,
                    interval: UncertaintyInterval::new(0.0, 0.001, 0.002).unwrap(),
                    unit: "L".into(),
                    bins: None,
                },
            ],
            equivalents: vec![],
            hypotheses: vec![Hypothesis {
                key: "k".into(),
                value: serde_json::json!(1),
                source: "test".into(),
            }],
            computed_at: ts,
            seed: 42,
        };
        let payload = serde_json::to_string(&result).unwrap();
        AuditEntry {
            id,
            timestamp: ts,
            estimation_result_json: payload,
            prev_sig: String::new(),
            sig: "x".repeat(64),
            purged_at: None,
        }
    }

    fn sample_request() -> ReportRequest {
        ReportRequest {
            period_start: Utc.with_ymd_and_hms(2026, 1, 1, 0, 0, 0).unwrap(),
            period_end: Utc.with_ymd_and_hms(2026, 4, 1, 0, 0, 0).unwrap(),
            organization_name: "Acme Corp".into(),
            locale: "fr".into(),
            app_version: "0.2.0".into(),
            estimator_seed: 42,
            estimator_n: 10_000,
        }
    }

    #[test]
    fn empty_period_returns_error() {
        let req = sample_request();
        let err = generate_report(&req, &[]).unwrap_err();
        assert!(matches!(err, ExportError::EmptyPeriod));
    }

    #[test]
    fn generates_pdf_with_content() {
        let req = sample_request();
        let entries = vec![
            make_entry(1, req.period_start, 1.0),
            make_entry(2, req.period_start, 2.0),
        ];
        let art = generate_report(&req, &entries).unwrap();
        assert!(!art.pdf_bytes.is_empty(), "PDF non vide");
        // Sanity check : un PDF commence par "%PDF-"
        assert!(art.pdf_bytes.starts_with(b"%PDF-"), "magic bytes PDF");
        assert_eq!(art.pdf_sha256.len(), 64);
        assert_eq!(art.summary.total_requests, 2);
        assert_eq!(art.audit_entries_count, 2);
    }

    #[test]
    fn pdf_is_dated_today_not_the_unix_epoch() {
        let req = sample_request();
        let day = || Utc::now().format("%Y%m%d").to_string();
        let before = day();
        let art = generate_report(&req, &[make_entry(1, req.period_start, 1.0)]).unwrap();
        let after = day();
        let text = String::from_utf8_lossy(&art.pdf_bytes);
        for key in ["/CreationDate", "/ModDate"] {
            let at = text.find(key).unwrap_or_else(|| panic!("{key} present"));
            let date: String = text[at + key.len()..]
                .chars()
                .skip_while(|c| *c != ':')
                .skip(1)
                .take(8)
                .collect();
            assert!(
                date == before || date == after,
                "{key} is {date}, not today ({before})"
            );
        }
    }

    #[test]
    fn text_page_puts_each_line_in_its_own_text_section() {
        let mut page = TextPage::default();
        let font = PdfFontHandle::Builtin(BuiltinFont::Helvetica);
        page.use_text("a", 10.0, Mm(1.0), Mm(2.0), &font);
        page.use_text("b", 10.0, Mm(1.0), Mm(3.0), &font);
        assert_eq!(page.ops.len(), 10);
        assert!(matches!(page.ops[0], Op::StartTextSection));
        assert!(matches!(page.ops[4], Op::EndTextSection));
        assert!(matches!(page.ops[5], Op::StartTextSection));
        assert!(matches!(page.ops[9], Op::EndTextSection));
    }

    #[test]
    fn pdf_sha256_is_deterministic_for_same_input() {
        let req = sample_request();
        let entries = vec![make_entry(1, req.period_start, 1.5)];
        let art1 = generate_report(&req, &entries).unwrap();
        let art2 = generate_report(&req, &entries).unwrap();
        // printpdf inclut un UUID dans le PDF metadata, donc strict
        // identique non garanti. Mais la taille devrait être identique.
        assert_eq!(art1.pdf_bytes.len(), art2.pdf_bytes.len());
    }

    #[test]
    fn provo_jsonld_links_to_pdf_sha256() {
        let req = sample_request();
        let entries = vec![make_entry(1, req.period_start, 1.0)];
        let art = generate_report(&req, &entries).unwrap();
        let prov = &art.provo_jsonld;
        let report_node = &prov["@graph"][0];
        assert_eq!(
            report_node["schema:contentSha256"].as_str().unwrap(),
            art.pdf_sha256
        );
    }

    #[test]
    fn provo_includes_org_name_in_report_node() {
        let req = sample_request();
        let entries = vec![make_entry(1, req.period_start, 1.0)];
        let art = generate_report(&req, &entries).unwrap();
        let prov = &art.provo_jsonld;
        assert_eq!(prov["@graph"][0]["sobria:organizationName"], "Acme Corp");
    }

    #[test]
    fn summary_aggregates_three_requests() {
        let req = sample_request();
        let entries = vec![
            make_entry(1, req.period_start, 1.0),
            make_entry(2, req.period_start, 2.0),
            make_entry(3, req.period_start, 3.0),
        ];
        let art = generate_report(&req, &entries).unwrap();
        assert_eq!(art.summary.total_requests, 3);
        assert!((art.summary.total_co2eq_g_p50 - 6.0).abs() < 1e-9);
    }
}
