//! Browser extension — pairing + ingestion (C27.5.c/d).

use chrono::Utc;
use tracing::info;

use super::lock_bare;
use crate::{
    dto::PairingCodeDto,
    error::{AppError, IpcError, IpcResult},
    state::AppState,
};

/// Generates (or regenerates) a 6-digit pairing code, 5 min TTL, single-use.
///
/// Stores the code in `state.pending_code` (overwrites any previous code).
/// The UI shows these 6 digits; the user types them into the extension,
/// which sends them through the bridge, and the desktop app then calls
/// `verify_pairing_code` to finish.
pub fn regenerate_pairing_code(state: &AppState) -> IpcResult<PairingCodeDto> {
    let code = crate::pairing::PendingCode::new();
    let dto = PairingCodeDto::from_pending(&code, Utc::now());
    *lock_bare(&state.pending_code)? = Some(code);
    info!(
        seconds_remaining = dto.seconds_remaining,
        "pairing: nouveau code généré"
    );
    Ok(dto)
}

/// Current status of the code (or `None` if no code is pending or it has expired).
pub fn get_pairing_code_status(state: &AppState) -> IpcResult<Option<PairingCodeDto>> {
    let guard = lock_bare(&state.pending_code)?;
    let now = Utc::now();
    Ok(guard
        .as_ref()
        .filter(|c| c.is_valid(now))
        .map(|c| PairingCodeDto::from_pending(c, now)))
}

/// Validates a code typed into the extension: if OK, generates the secret,
/// persists the pairing in `device_pairings`, consumes the `pending_code`, and
/// returns the secret in clear text (the only time it leaves the app).
pub fn verify_pairing_code(
    state: &AppState,
    code: &str,
    fingerprint: &str,
) -> IpcResult<crate::dto::PairingSecretDto> {
    if fingerprint.trim().is_empty() {
        return Err(IpcError::invalid_request("fingerprint vide"));
    }
    let now = Utc::now();
    let mut guard = lock_bare(&state.pending_code)?;
    let pending = guard.as_ref().ok_or_else(|| {
        IpcError::invalid_request("aucun code de pairing en attente — régénère un code")
    })?;
    crate::pairing::verify_code(pending, code, now).map_err(|e| match e {
        crate::pairing::PairingError::Malformed => {
            IpcError::invalid_request("code à 6 chiffres requis")
        },
        crate::pairing::PairingError::InvalidCode => {
            IpcError::new("invalid_pairing_code", "code invalide ou expiré")
        },
    })?;
    // Code OK: generate the secret and persist it.
    let secret = crate::pairing::PairingSecret::new();
    let store = lock_bare(&state.extension_store)?;
    let pairing_id = store
        .record_pairing(fingerprint, &secret)
        .map_err(AppError::from)?;
    // Consume the code (single-use).
    *guard = None;
    info!(%pairing_id, fingerprint, "pairing: validé + persisté");
    Ok(crate::dto::PairingSecretDto {
        pairing_id,
        secret_hex: secret.secret_hex,
    })
}

/// Lists the pairings (revoked ones included, for audit).
pub fn list_pairings(state: &AppState) -> IpcResult<Vec<crate::dto::PairingDto>> {
    let store = lock_bare(&state.extension_store)?;
    let rows = store.list_pairings().map_err(AppError::from)?;
    Ok(rows.iter().map(Into::into).collect())
}

/// Marks a pairing as revoked (the row stays, for audit).
pub fn revoke_pairing(state: &AppState, id: &str) -> IpcResult<()> {
    let store = lock_bare(&state.extension_store)?;
    store.revoke_pairing(id).map_err(AppError::from)?;
    info!(%id, "pairing: révoqué");
    Ok(())
}

/// Lists the events ingested from the extension (paginated).
pub fn list_extension_events(
    state: &AppState,
    limit: u32,
    offset: u32,
) -> IpcResult<Vec<crate::dto::ExtensionEventDto>> {
    let store = lock_bare(&state.extension_store)?;
    let rows = store
        .list_events(limit as usize, offset as usize)
        .map_err(AppError::from)?;
    Ok(rows.iter().map(Into::into).collect())
}

/// Drains the `~/.sobria/spool/incoming.jsonl` spool produced by `sobria-bridge`.
/// Returns the number of ingested events. Called manually or by a timer.
pub fn drain_extension_spool(state: &AppState) -> IpcResult<usize> {
    let spool_path =
        sobria_bridge::spool_path().map_err(|e| AppError::Internal(format!("{e:#}")))?;
    let store = lock_bare(&state.extension_store)?;
    let n = crate::extension_store::drain_spool(&store, &spool_path).map_err(AppError::from)?;
    if n > 0 {
        info!(count = n, "extension: spool drainé");
    }
    Ok(n)
}
