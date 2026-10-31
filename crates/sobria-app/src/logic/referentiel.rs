//! Gold referential (C26.5 — medallion pipeline).

use crate::{dto::ReferentielStatusDto, error::IpcResult};

/// Status of the Gold referential (`data/gold/referentiel.sqlite` or the path
/// set by `SOBRIA_REFERENTIEL_PATH`). Always OK: if the file is missing,
/// returns an `available=false` status with an explicit message
/// rather than an IPC error, so the UI can offer a
/// reload action.
pub fn get_referentiel_status() -> IpcResult<ReferentielStatusDto> {
    Ok(referentiel_status_dto())
}

/// Tries to reload the referential via `dvc pull`. If `dvc` is missing or
/// fails, returns `success: false` with the error message for the UI.
pub fn reload_referentiel() -> IpcResult<crate::dto::ReferentielReloadResultDto> {
    use crate::dto::ReferentielReloadResultDto;

    let pull = std::process::Command::new("dvc").arg("pull").output();
    match pull {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            // Cap at ~4 KB so the UI is not flooded.
            let mut combined = format!("{stdout}\n{stderr}");
            if combined.len() > 4096 {
                combined.truncate(4096);
                combined.push_str("\n…(tronqué)");
            }
            let status = referentiel_status_dto();
            let success = out.status.success() && status.available;
            let message = if success {
                "dvc pull terminé, référentiel rechargé.".to_string()
            } else if !out.status.success() {
                format!(
                    "dvc pull a échoué (code {:?}). Voir docs/operations/dvc.md.",
                    out.status.code()
                )
            } else {
                "dvc pull OK mais le référentiel reste indisponible.".to_string()
            };
            Ok(ReferentielReloadResultDto {
                success,
                message,
                dvc_output: combined,
                status: Some(status),
            })
        },
        Err(e) => Ok(ReferentielReloadResultDto {
            success: false,
            message: format!(
                "Impossible de lancer `dvc pull` : {e}. Installez DVC (`pip install dvc`) ou \
                 lancez manuellement `cargo run -p sobria-ingest -- pipeline run`."
            ),
            dvc_output: String::new(),
            status: Some(referentiel_status_dto()),
        }),
    }
}

/// Internal helper: opens the referential and builds a DTO. Turns errors
/// into `available=false` instead of propagating them, and lets the UI
/// decide.
fn referentiel_status_dto() -> ReferentielStatusDto {
    match sobria_referentiel::load() {
        Ok(r) => match r.status() {
            Ok(s) => ReferentielStatusDto {
                available: true,
                message: "Référentiel Gold opérationnel.".to_string(),
                version: s.version,
                snapshot_date: s.snapshot_date.to_rfc3339(),
                sha256: s.sha256,
                source_count: s.source_count,
                model_count: s.model_count,
                path: s.path,
            },
            Err(e) => ReferentielStatusDto::unavailable(
                format!("Référentiel ouvert mais lecture KO : {e}"),
                String::new(),
            ),
        },
        Err(e) => ReferentielStatusDto::unavailable(
            format!("{e}"),
            std::env::var(sobria_referentiel::REFERENTIEL_PATH_ENV)
                .unwrap_or_else(|_| sobria_referentiel::DEFAULT_REFERENTIEL_PATH.to_string()),
        ),
    }
}
