//! CSV batch (C21 — M18).

use tracing::{debug, info};

use super::{estimate_prompt, find_indicator_dto, user_default_method};
use crate::{
    batch,
    dto::{
        BatchAggregateDto, BatchModelAggregateDto, BatchRequestDto, BatchResultDto,
        EstimationRequestDto, EstimationResultDto,
    },
    error::{IpcError, IpcResult},
    state::AppState,
};

fn batch_error_to_ipc(e: batch::BatchError) -> IpcError {
    match e {
        batch::BatchError::Format(m) => IpcError::invalid_request(m),
        batch::BatchError::TooManyRejections {
            rejected,
            total,
            limit_pct,
        } => IpcError::invalid_request(format!(
            "{rejected}/{total} lignes rejetées (> {limit_pct}%) — vérifiez le format"
        )),
        batch::BatchError::EmptyBatch => {
            IpcError::invalid_request("le CSV ne contient aucune ligne de données")
        },
        batch::BatchError::Io(e) => IpcError::new("io_error", e.to_string()),
        // FileNotFound, TooManyRows, Csv: the IPC text is the Display text.
        other => IpcError::invalid_request(other.to_string()),
    }
}

/// Totals, mean, min and max of the batch rows' p50 values.
#[allow(clippy::cast_precision_loss)] // batch ≤ 1000 rows, fine for f64
fn batch_aggregate(output_rows: &[batch::BatchOutputRow]) -> BatchAggregateDto {
    if output_rows.is_empty() {
        return BatchAggregateDto {
            total_co2eq_g_p50: 0.0,
            total_energy_wh_p50: 0.0,
            total_water_l_p50: 0.0,
            avg_co2eq_g_p50: 0.0,
            min_co2eq_g_p50: 0.0,
            max_co2eq_g_p50: 0.0,
        };
    }
    let total_co2: f64 = output_rows.iter().map(|r| r.co2eq_p50_g).sum();
    let total_energy: f64 = output_rows.iter().map(|r| r.energy_wh_p50).sum();
    let total_water: f64 = output_rows.iter().map(|r| r.water_l_p50).sum();
    let n = output_rows.len() as f64;
    let min_co2 = output_rows
        .iter()
        .map(|r| r.co2eq_p50_g)
        .fold(f64::INFINITY, f64::min);
    let max_co2 = output_rows
        .iter()
        .map(|r| r.co2eq_p50_g)
        .fold(f64::NEG_INFINITY, f64::max);
    BatchAggregateDto {
        total_co2eq_g_p50: total_co2,
        total_energy_wh_p50: total_energy,
        total_water_l_p50: total_water,
        avg_co2eq_g_p50: total_co2 / n,
        min_co2eq_g_p50: min_co2,
        max_co2eq_g_p50: max_co2,
    }
}

/// Runs a CSV batch: parse → estimate per row → aggregate + optional export.
#[allow(clippy::cast_precision_loss)] // batch ≤ 1000 rows, fine for f64
pub fn run_batch_from_csv(req: BatchRequestDto, state: &AppState) -> IpcResult<BatchResultDto> {
    let input_path = std::path::PathBuf::from(&req.input_csv_path);
    let rows = batch::parse_csv(&input_path).map_err(batch_error_to_ipc)?;
    let total_input = rows.len();

    let mut output_rows: Vec<batch::BatchOutputRow> = Vec::with_capacity(total_input);
    let mut rejected: u32 = 0;
    let mut first_audit_id: i64 = i64::MAX;
    let mut last_audit_id: i64 = 0;
    // Read once: the loop only writes the last-used datacenter preference.
    let user_method = user_default_method(state);

    for (idx, row) in rows.into_iter().enumerate() {
        let est_req = EstimationRequestDto {
            model_id: row.model_id.clone(),
            tokens_in: row.tokens_in,
            tokens_out_estimated: row.tokens_out,
            datacenter_id: row.datacenter_id.clone(),
            method: Some(user_method),
            modalities: None,
            overhead: None,
        };
        match estimate_prompt(est_req, state) {
            Ok(dto) => {
                let co2 = pick_indicator_dto(&dto, "co2eq");
                let energy = pick_indicator_dto(&dto, "energy");
                let water = pick_indicator_dto(&dto, "water");
                let row_idx = u32::try_from(idx + 1).unwrap_or(u32::MAX);
                first_audit_id = first_audit_id.min(dto.audit_id);
                last_audit_id = last_audit_id.max(dto.audit_id);
                output_rows.push(batch::BatchOutputRow {
                    row_index: row_idx,
                    model_id: row.model_id,
                    tokens_in: row.tokens_in,
                    tokens_out: row.tokens_out,
                    datacenter_id: row.datacenter_id,
                    co2eq_p5_g: co2.0,
                    co2eq_p50_g: co2.1,
                    co2eq_p95_g: co2.2,
                    energy_wh_p50: energy.1,
                    water_l_p50: water.1,
                    audit_id: dto.audit_id,
                });
            },
            Err(e) => {
                rejected = rejected.saturating_add(1);
                debug!(idx, error = %e.message, "batch: row rejected");
            },
        }
    }

    batch::check_rejection_ratio(rejected as usize, total_input).map_err(batch_error_to_ipc)?;

    let aggregate = batch_aggregate(&output_rows);
    let by_model = aggregate_by_model(&output_rows);

    // Optional export
    let output_path = if let Some(out) = req.output_csv_path {
        let path = std::path::PathBuf::from(&out);
        batch::export_results_csv(&path, &output_rows).map_err(batch_error_to_ipc)?;
        Some(out)
    } else {
        None
    };

    let rows_processed = u32::try_from(output_rows.len()).unwrap_or(u32::MAX);
    info!(rows_processed, rejected, total_input, "batch CSV terminé");

    Ok(BatchResultDto {
        rows_processed,
        rows_rejected: rejected,
        aggregate,
        by_model,
        output_csv_path: output_path,
        first_audit_id: if first_audit_id == i64::MAX {
            0
        } else {
            first_audit_id
        },
        last_audit_id,
    })
}

/// Extracts (P5, P50, P95) of an indicator from the DTO (by its string id).
fn pick_indicator_dto(dto: &EstimationResultDto, id: &str) -> (f64, f64, f64) {
    find_indicator_dto(dto, id).map_or((0.0, 0.0, 0.0), |i| (i.p5, i.p50, i.p95))
}

fn aggregate_by_model(rows: &[batch::BatchOutputRow]) -> Vec<BatchModelAggregateDto> {
    use std::collections::HashMap;
    let mut grouped: HashMap<String, (u32, f64)> = HashMap::new();
    for r in rows {
        let entry = grouped.entry(r.model_id.clone()).or_insert((0, 0.0));
        entry.0 = entry.0.saturating_add(1);
        entry.1 += r.co2eq_p50_g;
    }
    let mut out: Vec<BatchModelAggregateDto> = grouped
        .into_iter()
        .map(|(model_id, (count, total))| BatchModelAggregateDto {
            model_id,
            count,
            total_co2eq_g_p50: total,
            avg_co2eq_g_p50: total / f64::from(count.max(1)),
        })
        .collect();
    out.sort_by(|a, b| {
        b.total_co2eq_g_p50
            .partial_cmp(&a.total_co2eq_g_p50)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    out
}
