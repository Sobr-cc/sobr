//! Audit ledger: integrity check, paginated listing and NDJSON export.

use tracing::{debug, info};

use super::{export_ledger_text, lock};
use crate::{
    dto::{AuditEntrySummaryDto, IntegrityReportDto},
    error::{AppError, IpcResult},
    state::AppState,
};

/// Verifies the integrity of the audit chain.
pub fn verify_audit(state: &AppState) -> IpcResult<IntegrityReportDto> {
    let ledger = lock(&state.ledger, "ledger")?;
    let report = ledger.verify_chain().map_err(AppError::from)?;
    debug!(
        valid = report.valid,
        total = report.total_entries,
        "audit: verify"
    );
    Ok((&report).into())
}

/// Lists the audit entries (summary) with pagination.
///
/// `limit` is capped at 1000 to protect the UI.
pub fn list_audit_entries(
    limit: u32,
    offset: u32,
    state: &AppState,
) -> IpcResult<Vec<AuditEntrySummaryDto>> {
    let limit = limit.min(1000) as usize;
    let offset = offset as usize;
    // The ledger's public API does not (yet) offer a paginated read, so
    // we export NDJSON in memory and slice it. Acceptable at the expected
    // volume (audit < 10⁵ entries). To be factored out in C10 if needed.
    let text = export_ledger_text(state)?;
    let mut out: Vec<AuditEntrySummaryDto> = Vec::with_capacity(limit);
    for (i, line) in text.lines().enumerate() {
        if i < offset {
            continue;
        }
        if out.len() >= limit {
            break;
        }
        let entry: sobria_audit::AuditEntry = serde_json::from_str(line).map_err(AppError::from)?;
        out.push(AuditEntrySummaryDto::from_entry(&entry));
    }
    Ok(out)
}

/// Exports the ledger as NDJSON to `path`. Returns the number of lines.
pub fn export_audit_ndjson(path: &std::path::Path, state: &AppState) -> IpcResult<usize> {
    use std::io::{BufWriter, Write};

    let ledger = lock(&state.ledger, "ledger")?;
    let mut file = BufWriter::new(std::fs::File::create(path).map_err(AppError::from)?);
    let n = ledger.export_ndjson(&mut file).map_err(AppError::from)?;
    // Mapped like a write error inside `export_ndjson`.
    file.flush()
        .map_err(|e| AppError::Audit(sobria_audit::AuditError::Io(e)))?;
    info!(path = %path.display(), lines = n, "audit: export NDJSON");
    Ok(n)
}
