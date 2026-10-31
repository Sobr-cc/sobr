//! User preferences stored in `referentiel.sqlite`.
//!
//! See [ADR-0010](../../docs/adr/ADR-0010-personas-and-module-gating.md)
//! and `docs/archive/chantiers/C10-onboarding-personas.md` §2.
//!
//! Schema:
//! ```sql
//! CREATE TABLE IF NOT EXISTS app_preferences (
//!     key         TEXT PRIMARY KEY,
//!     value       TEXT NOT NULL,        -- serialized JSON
//!     updated_at  TEXT NOT NULL          -- RFC 3339 UTC
//! );
//! ```
//!
//! Reserved keys: `persona`, `enabled_modules`, `onboarded`, `lang`,
//! `default_method`, `also_show_methods`, `default_datacenter_id` (the `KEY_*`
//! constants). Consumers ignore any other key (forward compatibility).

use std::path::Path;

use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use sobria_core::{EmpreinteMethod, ModuleId, Persona};
use tracing::info;

use crate::error::AppError;

/// Key of the `app_preferences` table that stores the current persona.
pub const KEY_PERSONA: &str = "persona";
/// Key that stores the JSON list of enabled modules.
pub const KEY_ENABLED_MODULES: &str = "enabled_modules";
/// Key that stores the onboarding flag (`"true"` / `"false"`).
pub const KEY_ONBOARDED: &str = "onboarded";
/// Key that stores the current language (`"fr"` / `"en"`).
pub const KEY_LANG: &str = "lang";
/// Key that stores the user's default methodology (C24).
pub const KEY_DEFAULT_METHOD: &str = "default_method";
/// Key that stores the JSON list of methodologies shown for reference (C24).
pub const KEY_ALSO_SHOW_METHODS: &str = "also_show_methods";
/// Key that stores the id of the default selected datacenter (C25).
pub const KEY_DEFAULT_DATACENTER: &str = "default_datacenter_id";

/// Internal form of the preferences before they cross IPC.
///
/// Each `Option<_>` means the key is missing in SQLite
/// (empty table or key never written). The caller (`logic::*`) then
/// applies the defaults.
#[derive(Debug, Clone, Default)]
pub struct StoredPreferences {
    /// Persona chosen during onboarding.
    pub persona: Option<Persona>,
    /// Enabled modules (may overlap or go beyond the persona bundle).
    pub enabled_modules: Option<Vec<ModuleId>>,
    /// True once the user has completed onboarding.
    pub onboarded: Option<bool>,
    /// Current language (`"fr"` or `"en"`).
    pub lang: Option<String>,
    /// Default methodology for computations (C24).
    /// `None` → falls back to `EmpreinteMethod::default_method()`
    /// (= `AfnorSobria`, the French standard).
    pub default_method: Option<EmpreinteMethod>,
    /// Extra methodologies shown in the "Voir aussi" panel
    /// next to the main result (C24). `None` → empty list (no
    /// reference compared by default, the user turns them on manually).
    pub also_show_methods: Option<Vec<EmpreinteMethod>>,
    /// Last datacenter the user selected in /estimate,
    /// /comparer or /simuler (C25). `None` = none chosen → the estimate
    /// uses the default `EstimationParams`.
    pub default_datacenter_id: Option<String>,
}

/// Preferences store backed by SQLite (`referentiel.sqlite`).
pub struct PreferencesStore {
    conn: Connection,
}

impl PreferencesStore {
    /// Opens (or creates) the `referentiel.sqlite` database and creates the
    /// `app_preferences` table if it does not exist.
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let conn = Connection::open(path)?;
        conn.execute_batch(
            r"
            PRAGMA journal_mode = WAL;
            CREATE TABLE IF NOT EXISTS app_preferences (
                key         TEXT PRIMARY KEY,
                value       TEXT NOT NULL,
                updated_at  TEXT NOT NULL
            );
            ",
        )?;
        info!(path = %path.display(), "préférences: ouverture du store");
        Ok(Self { conn })
    }

    /// Reads every reserved key (one query per key).
    pub fn read_all(&self) -> Result<StoredPreferences, AppError> {
        let persona = self
            .read_raw(KEY_PERSONA)?
            .map(|v| serde_json::from_str::<Persona>(&v))
            .transpose()
            .map_err(AppError::from)?;
        let enabled_modules = self
            .read_raw(KEY_ENABLED_MODULES)?
            .map(|v| serde_json::from_str::<Vec<ModuleId>>(&v))
            .transpose()
            .map_err(AppError::from)?;
        let onboarded = self
            .read_raw(KEY_ONBOARDED)?
            .map(|v| matches!(v.as_str(), "true"));
        let lang = self.read_raw(KEY_LANG)?;
        let default_method = self
            .read_raw(KEY_DEFAULT_METHOD)?
            .map(|v| serde_json::from_str::<EmpreinteMethod>(&v))
            .transpose()
            .map_err(AppError::from)?;
        let also_show_methods = self
            .read_raw(KEY_ALSO_SHOW_METHODS)?
            .map(|v| serde_json::from_str::<Vec<EmpreinteMethod>>(&v))
            .transpose()
            .map_err(AppError::from)?;
        let default_datacenter_id = self.read_raw(KEY_DEFAULT_DATACENTER)?;
        Ok(StoredPreferences {
            persona,
            enabled_modules,
            onboarded,
            lang,
            default_method,
            also_show_methods,
            default_datacenter_id,
        })
    }

    /// Writes the persisted preferences in a single transaction.
    ///
    /// `Option` semantics:
    /// - `persona`: `None` deletes the key (legacy behavior, opt-in).
    /// - Every other `Option` field: `None` **leaves the existing key
    ///   unchanged**. Use the dedicated setters (`set_default_datacenter_id`,
    ///   etc.) to clear a specific key.
    pub fn write_all(&mut self, prefs: &StoredPreferences) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let tx = self.conn.transaction()?;

        // persona: null → DELETE, otherwise JSON UPSERT.
        match &prefs.persona {
            Some(p) => {
                let v = serde_json::to_string(p)?;
                upsert(&tx, KEY_PERSONA, &v, &now)?;
            },
            None => delete(&tx, KEY_PERSONA)?,
        }

        // enabled_modules
        if let Some(modules) = &prefs.enabled_modules {
            let v = serde_json::to_string(modules)?;
            upsert(&tx, KEY_ENABLED_MODULES, &v, &now)?;
        }

        // onboarded
        if let Some(flag) = prefs.onboarded {
            let v = if flag { "true" } else { "false" };
            upsert(&tx, KEY_ONBOARDED, v, &now)?;
        }

        // lang
        if let Some(lang) = &prefs.lang {
            upsert(&tx, KEY_LANG, lang, &now)?;
        }

        // default_method (C24)
        if let Some(method) = &prefs.default_method {
            let v = serde_json::to_string(method)?;
            upsert(&tx, KEY_DEFAULT_METHOD, &v, &now)?;
        }

        // also_show_methods (C24)
        if let Some(methods) = &prefs.also_show_methods {
            let v = serde_json::to_string(methods)?;
            upsert(&tx, KEY_ALSO_SHOW_METHODS, &v, &now)?;
        }

        // default_datacenter_id (C25)
        if let Some(id) = &prefs.default_datacenter_id {
            upsert(&tx, KEY_DEFAULT_DATACENTER, id, &now)?;
        }

        tx.commit()?;
        Ok(())
    }

    fn read_raw(&self, key: &str) -> Result<Option<String>, AppError> {
        let v: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM app_preferences WHERE key = ?1",
                params![key],
                |r| r.get(0),
            )
            .optional()?;
        Ok(v)
    }

    fn write_raw(&mut self, key: &str, value: &str) -> Result<(), AppError> {
        let now = Utc::now().to_rfc3339();
        let tx = self.conn.transaction()?;
        upsert(&tx, key, value, &now)?;
        tx.commit()?;
        Ok(())
    }

    fn delete_raw(&mut self, key: &str) -> Result<(), AppError> {
        let tx = self.conn.transaction()?;
        delete(&tx, key)?;
        tx.commit()?;
        Ok(())
    }

    /// Persists the default datacenter id (C25). `None` clears the key.
    pub fn set_default_datacenter_id(&mut self, id: Option<&str>) -> Result<(), AppError> {
        match id {
            Some(v) => self.write_raw(KEY_DEFAULT_DATACENTER, v),
            None => self.delete_raw(KEY_DEFAULT_DATACENTER),
        }
    }
}

fn upsert(
    tx: &rusqlite::Transaction<'_>,
    key: &str,
    value: &str,
    now: &str,
) -> Result<(), AppError> {
    tx.execute(
        "INSERT INTO app_preferences (key, value, updated_at) \
         VALUES (?1, ?2, ?3) \
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        params![key, value, now],
    )?;
    Ok(())
}

fn delete(tx: &rusqlite::Transaction<'_>, key: &str) -> Result<(), AppError> {
    tx.execute("DELETE FROM app_preferences WHERE key = ?1", params![key])?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn open_temp() -> (tempfile::TempDir, PreferencesStore) {
        let tmp = tempfile::tempdir().unwrap();
        let store = PreferencesStore::open(&tmp.path().join("referentiel.sqlite")).unwrap();
        (tmp, store)
    }

    #[test]
    fn empty_store_returns_all_none() {
        let (_tmp, store) = open_temp();
        let prefs = store.read_all().unwrap();
        assert!(prefs.persona.is_none());
        assert!(prefs.enabled_modules.is_none());
        assert!(prefs.onboarded.is_none());
        assert!(prefs.lang.is_none());
        assert!(prefs.default_method.is_none());
        assert!(prefs.also_show_methods.is_none());
    }

    #[test]
    fn write_then_read_round_trip() {
        let (_tmp, mut store) = open_temp();
        let written = StoredPreferences {
            persona: Some(Persona::Enterprise),
            enabled_modules: Some(vec![ModuleId::M1, ModuleId::M22]),
            onboarded: Some(true),
            lang: Some("fr".into()),
            default_method: Some(EmpreinteMethod::EcoLogits),
            also_show_methods: Some(vec![EmpreinteMethod::AfnorSobria]),
            default_datacenter_id: Some("ovh-gra-gravelines".into()),
        };
        store.write_all(&written).unwrap();
        let read = store.read_all().unwrap();
        assert_eq!(read.persona, Some(Persona::Enterprise));
        assert_eq!(
            read.enabled_modules,
            Some(vec![ModuleId::M1, ModuleId::M22])
        );
        assert_eq!(read.onboarded, Some(true));
        assert_eq!(read.lang.as_deref(), Some("fr"));
        assert_eq!(read.default_method, Some(EmpreinteMethod::EcoLogits));
        assert_eq!(
            read.also_show_methods,
            Some(vec![EmpreinteMethod::AfnorSobria])
        );
        assert_eq!(
            read.default_datacenter_id.as_deref(),
            Some("ovh-gra-gravelines")
        );
    }

    #[test]
    fn write_overwrites_previous() {
        let (_tmp, mut store) = open_temp();
        store
            .write_all(&StoredPreferences {
                persona: Some(Persona::Student),
                enabled_modules: Some(vec![ModuleId::M1]),
                onboarded: Some(false),
                lang: Some("fr".into()),
                ..StoredPreferences::default()
            })
            .unwrap();
        store
            .write_all(&StoredPreferences {
                persona: Some(Persona::Researcher),
                enabled_modules: Some(vec![ModuleId::M1, ModuleId::M17]),
                onboarded: Some(true),
                lang: Some("en".into()),
                ..StoredPreferences::default()
            })
            .unwrap();
        let read = store.read_all().unwrap();
        assert_eq!(read.persona, Some(Persona::Researcher));
        assert_eq!(
            read.enabled_modules,
            Some(vec![ModuleId::M1, ModuleId::M17])
        );
        assert_eq!(read.onboarded, Some(true));
        assert_eq!(read.lang.as_deref(), Some("en"));
    }

    #[test]
    fn write_persona_none_deletes_row() {
        let (_tmp, mut store) = open_temp();
        store
            .write_all(&StoredPreferences {
                persona: Some(Persona::Enterprise),
                ..StoredPreferences::default()
            })
            .unwrap();
        assert!(store.read_all().unwrap().persona.is_some());
        store.write_all(&StoredPreferences::default()).unwrap();
        assert!(store.read_all().unwrap().persona.is_none());
    }

    #[test]
    fn default_datacenter_id_round_trip() {
        let (_tmp, mut store) = open_temp();
        assert!(store.read_all().unwrap().default_datacenter_id.is_none());

        store
            .set_default_datacenter_id(Some("aws-us-east-1"))
            .unwrap();
        let back = store.read_all().unwrap().default_datacenter_id;
        assert_eq!(back.as_deref(), Some("aws-us-east-1"));

        // Reverting to None must also persist.
        store.set_default_datacenter_id(None).unwrap();
        assert!(store.read_all().unwrap().default_datacenter_id.is_none());
    }

    #[test]
    fn store_persists_across_reopen() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("referentiel.sqlite");
        {
            let mut s = PreferencesStore::open(&path).unwrap();
            s.write_all(&StoredPreferences {
                persona: Some(Persona::ProTech),
                enabled_modules: Some(vec![ModuleId::M1, ModuleId::M3]),
                onboarded: Some(true),
                lang: Some("fr".into()),
                default_method: Some(EmpreinteMethod::EcoLogits),
                ..StoredPreferences::default()
            })
            .unwrap();
        }
        let s2 = PreferencesStore::open(&path).unwrap();
        let p = s2.read_all().unwrap();
        assert_eq!(p.persona, Some(Persona::ProTech));
        assert_eq!(p.onboarded, Some(true));
        assert_eq!(p.default_method, Some(EmpreinteMethod::EcoLogits));
    }
}
