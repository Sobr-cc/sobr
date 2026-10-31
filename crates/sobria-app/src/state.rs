//! State shared by the IPC commands.
//!
//! `AppState` is built once when Tauri starts and passed to
//! each command through `tauri::State<'_, AppState>`. The `AuditLedger`
//! sits behind a `Mutex` because writes (`append`) must be
//! serialized to keep the SHA-256 chain intact.

use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};

use sobria_audit::AuditLedger;
use sobria_estimator::MonteCarloEngine;
use tracing::info;

use crate::{
    error::AppError, extension_store::ExtensionStore, goals_store::PersonalGoalsStore,
    pairing::PendingCode, preferences_store::PreferencesStore, project_store::ProjectStore,
    team_settings::TeamSettingsStore,
};

/// Shared application state.
pub struct AppState {
    /// Data root (`~/.local/share/sobria` on Linux, etc.).
    pub data_root: PathBuf,
    /// Absolute path of the SQLite ledger.
    pub audit_path: PathBuf,
    /// Absolute path of the referentiel database (preferences + future catalogs).
    pub referentiel_path: PathBuf,
    /// Audit ledger (all writes serialized).
    pub ledger: Mutex<AuditLedger>,
    /// User preferences store — see ADR-0010.
    pub preferences: Mutex<PreferencesStore>,
    /// Eco-budget goals store — see brief C19 (M25).
    pub goals: Mutex<PersonalGoalsStore>,
    /// Scientific projects store — see brief C20 (M17).
    pub projects: Mutex<ProjectStore>,
    /// Monte-Carlo engine (immutable, shareable).
    pub estimator: MonteCarloEngine,
    /// Extension pairing code waiting to be used (C27.5.c).
    /// `None` until a pairing starts, or once the previous one is used or expired.
    pub pending_code: Mutex<Option<PendingCode>>,
    /// SQLite store for extension pairings + events (C27.5.d).
    pub extension_store: Mutex<ExtensionStore>,
    /// SQLite KV store for the self-hosted Team Mode (C28.6).
    pub team_settings: Mutex<TeamSettingsStore>,
}

impl AppState {
    /// Builds an `AppState`: the data root, the audit ledger and every store
    /// (preferences, goals, projects, extension, team settings).
    ///
    /// If `data_root` is `None`, `dirs::data_dir() / "sobria"` is used.
    pub fn init(data_root: Option<PathBuf>) -> Result<Self, AppError> {
        let data_root = data_root
            .or_else(|| dirs::data_dir().map(|d| d.join("sobria")))
            .ok_or_else(|| {
                AppError::Internal("impossible de déterminer la data dir système".into())
            })?;
        std::fs::create_dir_all(&data_root)?;

        let audit_path = data_root.join("audit.sqlite");
        info!(path = %audit_path.display(), "audit: ouverture du ledger");
        let ledger = AuditLedger::open(&audit_path)?;

        let referentiel_path = data_root.join("referentiel.sqlite");
        info!(path = %referentiel_path.display(), "préférences: ouverture du référentiel");
        let preferences = PreferencesStore::open(&referentiel_path)?;
        let goals = PersonalGoalsStore::open(&referentiel_path)?;
        let projects = ProjectStore::open(&referentiel_path)?;
        let extension_store = ExtensionStore::open(&referentiel_path)
            .map_err(|e| AppError::Internal(format!("extension_store: {e}")))?;
        let team_settings = TeamSettingsStore::open(&referentiel_path)
            .map_err(|e| AppError::Internal(format!("team_settings: {e}")))?;

        let estimator = MonteCarloEngine::default();
        Ok(Self {
            data_root,
            audit_path,
            referentiel_path,
            ledger: Mutex::new(ledger),
            preferences: Mutex::new(preferences),
            goals: Mutex::new(goals),
            projects: Mutex::new(projects),
            estimator,
            pending_code: Mutex::new(None),
            extension_store: Mutex::new(extension_store),
            team_settings: Mutex::new(team_settings),
        })
    }

    /// Builds an `AppState` in an explicit folder (handy for tests).
    pub fn init_in(dir: &Path) -> Result<Self, AppError> {
        Self::init(Some(dir.to_path_buf()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn init_in_creates_audit_sqlite() {
        let tmp = tempfile::tempdir().unwrap();
        let state = AppState::init_in(tmp.path()).unwrap();
        assert!(state.audit_path.exists());
        assert_eq!(state.data_root, tmp.path());
        // Ledger open: len() == 0
        let ledger = state.ledger.lock().unwrap();
        assert_eq!(ledger.len().unwrap(), 0);
    }

    #[test]
    fn init_in_creates_referentiel_sqlite() {
        let tmp = tempfile::tempdir().unwrap();
        let state = AppState::init_in(tmp.path()).unwrap();
        assert!(state.referentiel_path.exists());
        // Preferences are empty at boot
        let store = state.preferences.lock().unwrap();
        let prefs = store.read_all().unwrap();
        assert!(prefs.persona.is_none());
        assert!(prefs.onboarded.is_none());
    }

    #[test]
    fn init_in_idempotent_on_existing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let _s1 = AppState::init_in(tmp.path()).unwrap();
        let _s2 = AppState::init_in(tmp.path()).unwrap();
        // No panic, no error.
    }
}
