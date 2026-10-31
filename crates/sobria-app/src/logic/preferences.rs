//! User preferences (C10 — ADR-0010).

use sobria_core::ModuleId;
use tracing::info;

use super::lock;
use crate::{
    dto::AppPreferencesDto,
    error::{IpcError, IpcResult},
    preferences_store::StoredPreferences,
    state::AppState,
};

/// Fetches the user preferences. Merges the persisted values
/// with the defaults (`AppPreferencesDto::defaults()`).
pub fn get_app_preferences(state: &AppState) -> IpcResult<AppPreferencesDto> {
    let store = lock(&state.preferences, "preferences")?;
    let stored = store.read_all()?;
    drop(store);
    let defaults = AppPreferencesDto::defaults();
    Ok(AppPreferencesDto {
        persona: stored.persona.or(defaults.persona),
        enabled_modules: stored.enabled_modules.unwrap_or(defaults.enabled_modules),
        onboarded: stored.onboarded.unwrap_or(defaults.onboarded),
        lang: stored.lang.unwrap_or(defaults.lang),
        default_method: stored.default_method.unwrap_or(defaults.default_method),
        also_show_methods: stored
            .also_show_methods
            .unwrap_or(defaults.also_show_methods),
        default_datacenter_id: stored
            .default_datacenter_id
            .or(defaults.default_datacenter_id),
    })
}

/// Persists the user preferences after strict validation.
///
/// Errors returned (all as `invalid_request`):
/// - `lang` is not in `{"fr", "en"}`,
/// - `enabled_modules` contains duplicates,
/// - `enabled_modules` is empty while `onboarded == true` (an onboarded
///   user must keep at least one module, M1 at the very least).
///
/// Note: invalid `Persona` / `ModuleId` variants are already
/// rejected by serde at deserialization, so there is no need to re-validate here.
pub fn set_app_preferences(prefs: AppPreferencesDto, state: &AppState) -> IpcResult<()> {
    validate_lang(&prefs.lang)?;
    validate_modules(&prefs.enabled_modules, prefs.onboarded)?;

    let stored = StoredPreferences {
        persona: prefs.persona,
        enabled_modules: Some(prefs.enabled_modules),
        onboarded: Some(prefs.onboarded),
        lang: Some(prefs.lang),
        default_method: Some(prefs.default_method),
        also_show_methods: Some(prefs.also_show_methods),
        // C25: `write_all` only does an `UPSERT` for `Some` and ignores
        // `None` (skip-on-None semantics documented in
        // `preferences_store::write_all`). For an IPC set that must reflect
        // an explicit removal (`None` in the DTO ⇒ row deleted), we
        // call `set_default_datacenter_id` after `write_all` below.
        // So the field stays `None` here to avoid a double UPSERT.
        default_datacenter_id: None,
    };

    let mut store = lock(&state.preferences, "preferences")?;
    store.write_all(&stored)?;
    // C25: the `AppPreferencesDto` DTO allows `default_datacenter_id = None`
    // to clear the key explicitly. Since `write_all` is SKIP-on-None for every
    // non-`persona` `Option` field (see the `write_all` doc), a `write_all`
    // call never deletes this key, so the explicit intent goes through
    // the dedicated setter `set_default_datacenter_id`.
    //
    // Note: this setter runs in its own transaction, so the
    // `write_all` + `set_default_datacenter_id` pair is NOT atomic. A process
    // kill between the two leaves `default_datacenter_id` out of sync with
    // the rest. This is an accepted trade-off: the window is around a µs and
    // the user stake (prefilling a picker) does not justify the complexity
    // of a "full-replacement" parameter on `write_all`. To revisit if a
    // class of operations that need atomicity appears.
    store.set_default_datacenter_id(prefs.default_datacenter_id.as_deref())?;
    info!(
        persona = ?stored.persona,
        onboarded = ?stored.onboarded,
        "préférences : mise à jour"
    );
    Ok(())
}

fn validate_lang(lang: &str) -> IpcResult<()> {
    if lang == "fr" || lang == "en" {
        Ok(())
    } else {
        Err(IpcError::invalid_request(format!(
            "lang '{lang}' inconnue, attendu 'fr' ou 'en'"
        )))
    }
}

fn validate_modules(modules: &[ModuleId], onboarded: bool) -> IpcResult<()> {
    if onboarded && modules.is_empty() {
        return Err(IpcError::invalid_request(
            "un utilisateur onboardé doit conserver au moins un module",
        ));
    }
    // Duplicates: a module may appear only once.
    let mut seen = std::collections::HashSet::new();
    for m in modules {
        if !seen.insert(*m) {
            return Err(IpcError::invalid_request(format!(
                "module {m:?} en doublon dans enabled_modules"
            )));
        }
    }
    Ok(())
}
