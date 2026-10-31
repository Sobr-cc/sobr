//! territoire_fr (C13 — M20 French territory).

use sobria_geoloc::{
    aggregate_by_region, find_site_by_code_iris, generate_sankey_fr, load_rte_mix,
    load_territoire_fr, TerritoireFrArtifact, TerritoireFrError,
};

use crate::{
    dto::{IndustrialSiteSummaryDto, RegionFrAggregateDto, SankeyDataDto},
    error::{IpcError, IpcResult},
    state::AppState,
};

/// Expected path of the ODRÉ JSON produced by `sobria-ingest`.
fn territoire_fr_path(state: &AppState) -> std::path::PathBuf {
    state.data_root.join("territoire_fr.json")
}

fn rte_mix_path(state: &AppState) -> std::path::PathBuf {
    state.data_root.join("rte_mix_fr.json")
}

fn territoire_or_data_not_ingested(e: TerritoireFrError) -> IpcError {
    match e {
        TerritoireFrError::NotFound(p) => IpcError::new(
            "data_not_ingested",
            format!(
                "données territoire FR absentes ({}). Lance : \
                 cargo run -p sobria-ingest -- fetch territoire-fr",
                p.display()
            ),
        ),
        TerritoireFrError::Json(e) => {
            IpcError::new("io_error", format!("territoire_fr.json corrompu : {e}"))
        },
        TerritoireFrError::Schema(m) => IpcError::new(
            "data_not_ingested",
            format!("territoire_fr.json non conforme : {m}"),
        ),
        TerritoireFrError::Io(e) => IpcError::new("io_error", e.to_string()),
    }
}

fn sankey_or_data_not_ingested(e: sobria_geoloc::SankeyFrError) -> IpcError {
    match e {
        sobria_geoloc::SankeyFrError::NotFound(p) => IpcError::new(
            "data_not_ingested",
            format!(
                "données RTE mix absentes ({}). Lance : \
                 cargo run -p sobria-ingest -- fetch rte-mix --year 2023",
                p.display()
            ),
        ),
        sobria_geoloc::SankeyFrError::Json(e) => {
            IpcError::new("io_error", format!("rte_mix_fr.json corrompu : {e}"))
        },
        sobria_geoloc::SankeyFrError::Schema(m) => IpcError::new(
            "data_not_ingested",
            format!("rte_mix_fr.json non conforme : {m}"),
        ),
        sobria_geoloc::SankeyFrError::Io(e) => IpcError::new("io_error", e.to_string()),
    }
}

fn load_or_err(state: &AppState) -> IpcResult<TerritoireFrArtifact> {
    load_territoire_fr(&territoire_fr_path(state)).map_err(territoire_or_data_not_ingested)
}

/// Paginated list of industrial sites (top by total consumption, descending).
pub fn list_industrial_sites_fr(
    limit: u32,
    offset: u32,
    state: &AppState,
) -> IpcResult<Vec<IndustrialSiteSummaryDto>> {
    let artifact = load_or_err(state)?;
    let limit = (limit as usize).min(1000);
    let offset = offset as usize;
    let dtos: Vec<IndustrialSiteSummaryDto> = artifact
        .industrial_sites
        .iter()
        .skip(offset)
        .take(limit)
        .map(Into::into)
        .collect();
    Ok(dtos)
}

/// Details of an IRIS site, looked up by its code.
pub fn get_industrial_site_fr(
    code_iris: &str,
    state: &AppState,
) -> IpcResult<IndustrialSiteSummaryDto> {
    let artifact = load_or_err(state)?;
    let site = find_site_by_code_iris(&artifact, code_iris)
        .ok_or_else(|| IpcError::new("not_found", format!("site IRIS '{code_iris}' inconnu")))?;
    Ok(site.into())
}

/// Aggregates the industrial sites by ISO region.
pub fn aggregate_industrial_sites_by_region(
    state: &AppState,
) -> IpcResult<Vec<RegionFrAggregateDto>> {
    let artifact = load_or_err(state)?;
    Ok(aggregate_by_region(&artifact)
        .iter()
        .map(Into::into)
        .collect())
}

/// Generates the national energy Sankey data from the loaded RTE mix.
pub fn sankey_fr_data(state: &AppState) -> IpcResult<SankeyDataDto> {
    let mix = load_rte_mix(&rte_mix_path(state)).map_err(sankey_or_data_not_ingested)?;
    let sankey = generate_sankey_fr(&mix);
    Ok((&sankey).into())
}
