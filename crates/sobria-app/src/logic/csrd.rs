//! CSRD/AGEC report (C14 — M22).

use tracing::info;

use super::{parse_rfc3339, read_all_audit_entries, APP_VERSION};
use crate::{
    dto::{CsrdReportRequestDto, CsrdReportResultDto},
    error::{AppError, IpcError, IpcResult},
    project_store,
    state::AppState,
};

/// Generates a CSRD/AGEC report for the requested period and writes the
/// `report.pdf` + `provo.jsonld` files into `output_dir`.
///
/// The period includes `period_start` and excludes `period_end`.
pub fn export_csrd_report(
    req: CsrdReportRequestDto,
    output_dir: &std::path::Path,
    state: &AppState,
) -> IpcResult<CsrdReportResultDto> {
    // 1. Parse the ISO 8601 dates.
    let period_start = parse_rfc3339(&req.period_start, "period_start")?;
    let period_end = parse_rfc3339(&req.period_end, "period_end")?;
    project_store::validate_period(period_start, period_end)?;
    if req.organization_name.trim().is_empty() {
        return Err(IpcError::invalid_request(
            "organization_name ne doit pas être vide",
        ));
    }
    if !matches!(req.locale.as_str(), "fr" | "en") {
        return Err(IpcError::invalid_request(format!(
            "locale '{}' non supportée (fr|en)",
            req.locale
        )));
    }

    // 2. Read every audit ledger entry.
    let entries = read_all_audit_entries(state)?;

    // 3. Generate the report.
    let export_req = sobria_export::ReportRequest {
        period_start,
        period_end,
        organization_name: req.organization_name,
        locale: req.locale,
        app_version: APP_VERSION.into(),
        estimator_seed: state.estimator.seed(),
        estimator_n: state.estimator.n(),
    };
    let artifacts = sobria_export::generate_report(&export_req, &entries).map_err(|e| match e {
        sobria_export::ExportError::EmptyPeriod => IpcError::new(
            "empty_period",
            "aucune entrée d'audit dans la période demandée",
        ),
        other => IpcError::new("export_error", other.to_string()),
    })?;

    // 4. Write the artefacts to disk.
    std::fs::create_dir_all(output_dir).map_err(AppError::from)?;
    let pdf_path = output_dir.join("report.pdf");
    let provo_path = output_dir.join("provo.jsonld");
    std::fs::write(&pdf_path, &artifacts.pdf_bytes).map_err(AppError::from)?;
    let provo_text =
        serde_json::to_string_pretty(&artifacts.provo_jsonld).map_err(AppError::from)?;
    std::fs::write(&provo_path, provo_text).map_err(AppError::from)?;

    info!(
        pdf = %pdf_path.display(),
        provo = %provo_path.display(),
        sha256 = %artifacts.pdf_sha256,
        "rapport CSRD généré"
    );

    Ok(CsrdReportResultDto {
        pdf_path: pdf_path.display().to_string(),
        provo_path: provo_path.display().to_string(),
        pdf_sha256: artifacts.pdf_sha256,
        audit_entries_count: artifacts.audit_entries_count,
        total_requests: artifacts.summary.total_requests,
        total_co2eq_g_p50: artifacts.summary.total_co2eq_g_p50,
        total_energy_wh_p50: artifacts.summary.total_energy_wh_p50,
        total_water_l_p50: artifacts.summary.total_water_l_p50,
    })
}
