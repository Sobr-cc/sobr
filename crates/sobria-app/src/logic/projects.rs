//! Projects + datasheet (C20 — M17 project footprint).

use tracing::info;

use super::{lock, parse_rfc3339, read_all_audit_entries};
use crate::{
    dto::{CompositionDto, CreateProjectDto, DatasheetDto, ProjectDto, UpdateProjectDto},
    error::{IpcError, IpcResult},
    project_store::{NewProject, Project, ProjectUpdate},
    state::AppState,
};

/// Lists all saved projects (descending ID order).
pub fn list_projects(state: &AppState) -> IpcResult<Vec<ProjectDto>> {
    let store = lock(&state.projects, "projects")?;
    let projects = store.list_all()?;
    Ok(projects.into_iter().map(Into::into).collect())
}

/// Fetches a project by its ID.
pub fn get_project(id: i64, state: &AppState) -> IpcResult<ProjectDto> {
    Ok(load_project(id, state)?.into())
}

/// Creates a new project and returns its persisted version.
pub fn create_project(req: CreateProjectDto, state: &AppState) -> IpcResult<ProjectDto> {
    let period_start = parse_rfc3339(&req.period_start, "period_start")?;
    let period_end = parse_rfc3339(&req.period_end, "period_end")?;
    let mut store = lock(&state.projects, "projects")?;
    let p = store.create(NewProject {
        name: req.name,
        description: req.description,
        period_start,
        period_end,
        tags: req.tags,
    })?;
    info!(id = p.id, name = %p.name, "project: created");
    Ok(p.into())
}

/// Updates an existing project (optional fields; dates cannot be changed).
pub fn update_project(id: i64, req: UpdateProjectDto, state: &AppState) -> IpcResult<ProjectDto> {
    let mut store = lock(&state.projects, "projects")?;
    let p = store.update(
        id,
        ProjectUpdate {
            name: req.name,
            description: req.description,
            tags: req.tags,
        },
    )?;
    Ok(p.into())
}

/// Deletes a project (idempotent).
pub fn delete_project(id: i64, state: &AppState) -> IpcResult<()> {
    let mut store = lock(&state.projects, "projects")?;
    store.delete(id)?;
    Ok(())
}

/// Generates the Gebru JSON-LD datasheet for an existing project.
pub fn generate_project_datasheet(id: i64, state: &AppState) -> IpcResult<DatasheetDto> {
    // 1. Fetch the project.
    let project = load_project(id, state)?;

    // 2. Keep the ledger entries that fall in the period.
    let all_entries = read_all_audit_entries(state)?;
    let in_period: Vec<sobria_audit::AuditEntry> = all_entries
        .into_iter()
        .filter(|e| e.timestamp >= project.period_start && e.timestamp < project.period_end)
        .collect();

    // 3. Build the datasheet.
    let meta = sobria_export::ProjectMeta {
        id: project.id,
        name: project.name.clone(),
        description: project.description.clone(),
        period_start: project.period_start,
        period_end: project.period_end,
        tags: project.tags.clone(),
        created_at: project.created_at,
    };
    let opts = sobria_export::DatasheetOptions::default();
    let artifact = sobria_export::build_datasheet(&meta, &in_period, &opts);

    info!(
        project_id = project.id,
        entries = in_period.len(),
        sha = %artifact.sha256,
        "datasheet généré"
    );

    Ok(DatasheetDto {
        project: project.into(),
        jsonld: artifact.jsonld,
        composition: CompositionDto::from(&artifact.composition),
        sha256: artifact.sha256,
    })
}

/// Fetches a project, or `not_found` if the id is unknown.
fn load_project(id: i64, state: &AppState) -> IpcResult<Project> {
    let store = lock(&state.projects, "projects")?;
    store
        .get(id)?
        .ok_or_else(|| IpcError::new("not_found", format!("projet {id} inconnu")))
}
