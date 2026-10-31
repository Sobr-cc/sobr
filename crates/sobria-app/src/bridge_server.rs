//! Socket server: receives the requests of `sobria-bridge` over a Unix socket
//! or a Windows named pipe, and routes them to the existing pairing /
//! ingestion logic.
//!
//! ## Protocol
//!
//! Length-prefixed JSON (uint32 LE + UTF-8 JSON), same as the stdio bridge:
//!   - the bridge sends `sobria_bridge::BridgeRequest`;
//!   - the app answers `sobria_bridge::BridgeResponse`.
//!
//! ## Offline fallback
//!
//! If the app is not running when the extension sends an estimate, the
//! bridge writes to the spool file (`~/.sobria/spool/incoming.jsonl`).
//! The app keeps draining the spool every 5 s in parallel (see
//! `logic::drain_extension_spool`) to pick up the offline writes.
//!
//! ## Security
//!
//! The Unix socket is bound to a local path (`$XDG_RUNTIME_DIR` or
//! `/tmp`) and inherits the fs permissions. The Windows named pipe is
//! created with `ServerOptions`; named pipes are global, not limited to the
//! current user. No network port, no network authentication: security
//! relies on the OS.
//!
//! The secret is checked in `dispatch_request` (Argon2id PHC) before
//! any event is inserted.

use anyhow::{Context, Result};
use sobria_bridge::{BridgeRequest, BridgeResponse};
use tauri::{AppHandle, Manager, Runtime};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use crate::{
    extension_store::{ExtensionEventInput, ExtensionStore},
    pairing::{verify_code, PairingSecret},
    state::AppState,
};

/// Payload upper limit, the bridge's own limit (1 MB).
const MAX_PAYLOAD_BYTES: usize = sobria_bridge::MAX_MESSAGE_BYTES;

/// Starts the socket / pipe server. Infinite loop: returns `Err` only if
/// the initial bind fails; per-connection errors are logged and ignored.
///
/// Takes the Tauri `AppHandle` to resolve `AppState` on each dispatch, so
/// `AppState` does not need an `Arc` wrapper in Tauri's managed state.
pub async fn run(app_handle: AppHandle) -> Result<()> {
    serve(app_handle, &sobria_bridge::default_socket_path()).await
}

/// Same as [`run`] on an explicit socket / pipe path. Test hook of SOB-188:
/// it lets a test start this very code on a private endpoint, with the
/// `AppHandle` of a mock app (hence the generic runtime), instead of copying
/// the loop of the server.
#[doc(hidden)]
pub async fn serve<R: Runtime>(app_handle: AppHandle<R>, path: &std::path::Path) -> Result<()> {
    #[cfg(unix)]
    {
        run_unix(app_handle, path).await
    }
    #[cfg(windows)]
    {
        run_windows(app_handle, path).await
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (app_handle, path);
        anyhow::bail!("plateforme non supportée pour le socket forward")
    }
}

#[cfg(unix)]
async fn run_unix<R: Runtime>(app_handle: AppHandle<R>, path: &std::path::Path) -> Result<()> {
    use tokio::net::UnixListener;
    // Removes a leftover from a previous crash. Otherwise `bind` would
    // fail with EADDRINUSE.
    let _ = std::fs::remove_file(path);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let listener =
        UnixListener::bind(path).with_context(|| format!("bind unix socket {}", path.display()))?;
    tracing::info!(path = %path.display(), "bridge_server: en écoute (Unix)");
    loop {
        match listener.accept().await {
            Ok((stream, _addr)) => {
                let app_handle = app_handle.clone();
                tokio::spawn(async move {
                    if let Err(e) = handle_stream(stream, &app_handle).await {
                        tracing::warn!(error = %e, "bridge_server: erreur connexion");
                    }
                });
            },
            Err(e) => {
                tracing::warn!(error = %e, "bridge_server: accept failed");
            },
        }
    }
}

#[cfg(windows)]
async fn run_windows<R: Runtime>(app_handle: AppHandle<R>, path: &std::path::Path) -> Result<()> {
    use tokio::net::windows::named_pipe::ServerOptions;
    let pipe_name = path.to_str().context("pipe path non-UTF-8")?.to_owned();
    tracing::info!(pipe = %pipe_name, "bridge_server: en écoute (Windows pipe)");
    // First instance: `first_pipe_instance(true)` blocks concurrent binds
    // to the same name.
    let mut server = ServerOptions::new()
        .first_pipe_instance(true)
        .create(&pipe_name)
        .with_context(|| format!("create named pipe {pipe_name}"))?;
    loop {
        server.connect().await.context("named pipe connect")?;
        // Prepares the next instance BEFORE spawning: Win32 requires a new
        // instance to be ready for the next client.
        let next = ServerOptions::new()
            .create(&pipe_name)
            .context("named pipe next instance")?;
        let active = std::mem::replace(&mut server, next);
        let app_handle = app_handle.clone();
        tokio::spawn(async move {
            if let Err(e) = handle_stream(active, &app_handle).await {
                tracing::warn!(error = %e, "bridge_server: erreur connexion (pipe)");
            }
        });
    }
}

/// Reads a length-prefixed request from `stream`, dispatches it, writes the response.
async fn handle_stream<S, R: Runtime>(mut stream: S, app_handle: &AppHandle<R>) -> Result<()>
where
    S: AsyncReadExt + AsyncWriteExt + Unpin,
{
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .await
        .context("read request length")?;
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > MAX_PAYLOAD_BYTES {
        anyhow::bail!("request > {MAX_PAYLOAD_BYTES} bytes: {len}");
    }
    let mut payload = vec![0u8; len];
    stream
        .read_exact(&mut payload)
        .await
        .context("read request payload")?;
    let req: BridgeRequest = serde_json::from_slice(&payload).context("decode BridgeRequest")?;
    // Tauri manages `AppState` directly (see main.rs): `state.inner()`
    // gives `&AppState`.
    let state = app_handle.state::<AppState>();
    let resp = dispatch_request(state.inner(), req);
    let bytes = serde_json::to_vec(&resp).context("encode BridgeResponse")?;
    let resp_len = u32::try_from(bytes.len()).context("response > u32")?;
    let mut frame = Vec::with_capacity(4 + bytes.len());
    frame.extend_from_slice(&resp_len.to_le_bytes());
    frame.extend_from_slice(&bytes);
    stream.write_all(&frame).await?;
    stream.flush().await?;
    Ok(())
}

/// Pure-logic dispatcher: no I/O. Testable in isolation.
///
/// - `Ping` → `{ pong: true }`.
/// - `Pair { code }` → consumes `pending_code` via `verify_code`, generates
///   a `PairingSecret`, persists it, returns `secret` + `pairing_id`.
/// - `Estimate { secret, payload }` → checks the secret (Argon2id) against
///   `device_pairings`, inserts an `ExtensionEventInput`.
/// - `Revoke { secret }` → finds the active pairing matching the secret
///   and revokes it.
pub fn dispatch_request(state: &AppState, req: BridgeRequest) -> BridgeResponse {
    let (req_id, reply) = match req {
        BridgeRequest::Ping { req_id } => {
            return BridgeResponse {
                pong: Some(true),
                ..BridgeResponse::ok(req_id)
            };
        },
        BridgeRequest::Pair { req_id, code } => (req_id, handle_pair(state, &code)),
        BridgeRequest::Estimate {
            req_id,
            secret,
            payload,
        } => (req_id, handle_estimate(state, &secret, &payload)),
        BridgeRequest::Revoke { req_id, secret } => (req_id, handle_revoke(state, &secret)),
    };
    match reply {
        // The handlers leave `req_id` empty: it is moved in here, not copied.
        Ok(reply) => BridgeResponse { req_id, ..reply },
        Err(msg) => BridgeResponse::err(req_id, msg),
    }
}

/// Synthetic fingerprint on the app side: the bridge request carries none, so
/// every browser shares it (one pairing per browser is SOB-119).
fn bridge_fingerprint() -> &'static str {
    "sobria-bridge-native-messaging"
}

fn handle_pair(state: &AppState, code: &str) -> Result<BridgeResponse, String> {
    let now = chrono::Utc::now();
    let mut guard = state
        .pending_code
        .lock()
        .map_err(|e| format!("state lock: {e}"))?;
    let pending = guard
        .as_ref()
        .ok_or("aucun code de pairing en attente — génère-en un depuis Sobr.ia /parametres")?;
    verify_code(pending, code, now).map_err(|e| e.to_string())?;
    let secret = PairingSecret::new();
    let fingerprint = bridge_fingerprint();
    let store = state
        .extension_store
        .lock()
        .map_err(|e| format!("store lock: {e}"))?;
    let pairing_id = store
        .record_pairing(fingerprint, &secret)
        .map_err(|e| format!("record pairing: {e}"))?;
    // Consumes the code (single-use).
    *guard = None;
    tracing::info!(%pairing_id, "socket_server: pairing validé via bridge");
    Ok(BridgeResponse {
        secret: Some(secret.secret_hex),
        pairing_id: Some(pairing_id),
        fingerprint: Some(fingerprint.into()),
        ..BridgeResponse::ok(String::new())
    })
}

fn handle_estimate(
    state: &AppState,
    secret: &str,
    payload: &serde_json::Value,
) -> Result<BridgeResponse, String> {
    let store = state
        .extension_store
        .lock()
        .map_err(|e| format!("store lock: {e}"))?;
    let pairing_id = verified_pairing(&store, secret, "secret invalide ou pairing révoqué")?;
    let estimate = payload
        .get("estimate")
        .ok_or("payload sans champ `estimate`")?;
    let input =
        ExtensionEventInput::from_estimate(pairing_id, chrono::Utc::now(), payload, estimate, 0);
    store
        .record_event(&input)
        .map_err(|e| format!("record_event: {e}"))?;
    Ok(BridgeResponse {
        pairing_id: Some(input.pairing_id),
        ..BridgeResponse::ok(String::new())
    })
}

fn handle_revoke(state: &AppState, secret: &str) -> Result<BridgeResponse, String> {
    let store = state
        .extension_store
        .lock()
        .map_err(|e| format!("store lock: {e}"))?;
    let pairing_id = verified_pairing(&store, secret, "secret invalide — déjà révoqué ?")?;
    store
        .revoke_pairing(&pairing_id)
        .map_err(|e| format!("revoke: {e}"))?;
    tracing::info!(%pairing_id, "socket_server: pairing révoqué via bridge");
    Ok(BridgeResponse {
        pairing_id: Some(pairing_id),
        ..BridgeResponse::ok(String::new())
    })
}

/// Id of the active pairing matching `secret`; `not_found` when there is none.
fn verified_pairing(
    store: &ExtensionStore,
    secret: &str,
    not_found: &str,
) -> Result<String, String> {
    store
        .verify_secret(bridge_fingerprint(), secret)
        .map_err(|e| format!("verify_secret: {e}"))?
        .ok_or_else(|| not_found.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::AppState;
    use sobria_bridge::BridgeRequest;

    fn fresh_state() -> (tempfile::TempDir, AppState) {
        let tmp = tempfile::tempdir().unwrap();
        let state = AppState::init_in(tmp.path()).unwrap();
        (tmp, state)
    }

    /// A 6-digit code that is guaranteed to differ from `code`.
    fn wrong_code_for(code: &str) -> String {
        let n: u32 = code.parse().expect("wrong_code_for expects a 6-digit code");
        format!("{:06}", (n + 1) % 1_000_000)
    }

    /// Installs a pending code that expired 10 minutes ago and returns that code.
    fn install_expired_code(state: &AppState) -> String {
        let expired = crate::pairing::PendingCode::with_expiry(
            chrono::Utc::now() - chrono::Duration::minutes(10),
        );
        let code = expired.code.clone();
        *state.pending_code.lock().unwrap() = Some(expired);
        code
    }

    /// Error message of a `Pair` that must be refused.
    #[track_caller]
    fn pair_error(state: &AppState, code: &str) -> String {
        let resp = dispatch_request(
            state,
            BridgeRequest::Pair {
                req_id: "r-pair".into(),
                code: code.into(),
            },
        );
        assert!(!resp.ok, "le Pair avec {code:?} aurait dû être refusé");
        resp.error.expect("une réponse refusée porte un message")
    }

    #[test]
    fn ping_returns_pong() {
        let (_tmp, state) = fresh_state();
        let resp = dispatch_request(
            &state,
            BridgeRequest::Ping {
                req_id: "r1".into(),
            },
        );
        assert!(resp.ok);
        assert_eq!(resp.pong, Some(true));
        assert!(resp.error.is_none());
    }

    #[test]
    fn pair_without_pending_code_returns_error() {
        let (_tmp, state) = fresh_state();
        let resp = dispatch_request(
            &state,
            BridgeRequest::Pair {
                req_id: "r1".into(),
                code: "123456".into(),
            },
        );
        assert!(!resp.ok);
        assert!(resp.error.unwrap().contains("aucun code"));
    }

    #[test]
    fn pair_with_valid_code_returns_secret_and_pairing_id() {
        let (_tmp, state) = fresh_state();
        // Generates a code.
        let dto = crate::logic::regenerate_pairing_code(&state).unwrap();
        let resp = dispatch_request(
            &state,
            BridgeRequest::Pair {
                req_id: "r1".into(),
                code: dto.code,
            },
        );
        assert!(resp.ok, "got error: {:?}", resp.error);
        assert_eq!(resp.secret.as_deref().unwrap_or("").len(), 64);
        assert!(resp.pairing_id.is_some());
        assert_eq!(resp.fingerprint.as_deref(), Some(bridge_fingerprint()));
    }

    #[test]
    fn pair_code_is_single_use() {
        let (_tmp, state) = fresh_state();
        let dto = crate::logic::regenerate_pairing_code(&state).unwrap();
        let first = dispatch_request(
            &state,
            BridgeRequest::Pair {
                req_id: "r-first".into(),
                code: dto.code.clone(),
            },
        );
        assert!(first.ok, "got error: {:?}", first.error);

        // The code is consumed: a second Pair with the same code finds no
        // pending code (rather than "code invalide").
        assert_eq!(
            pair_error(&state, &dto.code),
            "aucun code de pairing en attente — génère-en un depuis Sobr.ia /parametres"
        );
    }

    #[test]
    fn pair_with_expired_pending_code_is_refused() {
        let (_tmp, state) = fresh_state();
        // The right code, but expired: same message as a wrong code.
        let code = install_expired_code(&state);
        assert_eq!(pair_error(&state, &code), "code invalide ou expiré");
    }

    #[test]
    fn pair_with_malformed_code_gets_the_six_digits_message() {
        let (_tmp, state) = fresh_state();
        let _ = crate::logic::regenerate_pairing_code(&state).unwrap();
        // No trimming and no Unicode digits: spaces, newlines and
        // full-width digits are refused as malformed.
        for malformed in [
            "",
            "12345",
            "1234567",
            "12345a",
            "abcdef",
            " 123456",
            "123456\n",
            "１２３４５６",
        ] {
            assert_eq!(
                pair_error(&state, malformed),
                "code à 6 chiffres requis",
                "code {malformed:?}"
            );
        }
    }

    #[test]
    fn pair_with_wrong_six_digit_code_gets_the_invalid_message() {
        let (_tmp, state) = fresh_state();
        let dto = crate::logic::regenerate_pairing_code(&state).unwrap();
        let wrong = wrong_code_for(&dto.code);
        assert_eq!(pair_error(&state, &wrong), "code invalide ou expiré");
    }

    #[test]
    fn pair_with_right_code_still_works_after_wrong_codes() {
        let (_tmp, state) = fresh_state();
        let dto = crate::logic::regenerate_pairing_code(&state).unwrap();
        let wrong = wrong_code_for(&dto.code);
        // There is no attempt limit today: adding one must change this test on purpose.
        for attempt in 1..=20 {
            assert_eq!(
                pair_error(&state, &wrong),
                "code invalide ou expiré",
                "failed attempt {attempt}"
            );
        }

        // Refusals do not consume the pending code, so the right code still pairs.
        let resp = dispatch_request(
            &state,
            BridgeRequest::Pair {
                req_id: "r-retry".into(),
                code: dto.code,
            },
        );
        assert!(resp.ok, "got error: {:?}", resp.error);
        assert_eq!(resp.secret.as_deref().unwrap_or("").len(), 64);
        assert!(resp.pairing_id.is_some());
    }

    #[test]
    fn pair_with_expired_code_keeps_it_pending() {
        let (_tmp, state) = fresh_state();
        let code = install_expired_code(&state);

        assert_eq!(
            pair_error(&state, &code),
            "code invalide ou expiré",
            "first attempt"
        );
        // The refusal does not consume the expired code: it is still the pending one,
        // so the second attempt gets the same message, not "aucun code de pairing".
        assert_eq!(
            state
                .pending_code
                .lock()
                .unwrap()
                .as_ref()
                .map(|p| p.code.as_str()),
            Some(code.as_str())
        );
        assert_eq!(
            pair_error(&state, &code),
            "code invalide ou expiré",
            "second attempt"
        );
    }

    #[test]
    fn refused_pair_records_no_pairing() {
        let (_tmp, state) = fresh_state();
        // Each refusal path is checked by its message, so a path that changes fails here.
        // No pending code.
        assert_eq!(
            pair_error(&state, "123456"),
            "aucun code de pairing en attente — génère-en un depuis Sobr.ia /parametres"
        );
        // Pending code: malformed, then wrong.
        let dto = crate::logic::regenerate_pairing_code(&state).unwrap();
        assert_eq!(pair_error(&state, "abc"), "code à 6 chiffres requis");
        let wrong = wrong_code_for(&dto.code);
        assert_eq!(pair_error(&state, &wrong), "code invalide ou expiré");
        // Expired code.
        let code = install_expired_code(&state);
        assert_eq!(pair_error(&state, &code), "code invalide ou expiré");

        let pairings = crate::logic::list_pairings(&state).unwrap();
        assert_eq!(pairings.len(), 0, "a refused Pair must record nothing");
    }

    #[test]
    fn estimate_with_wrong_secret_returns_error() {
        let (_tmp, state) = fresh_state();
        let resp = dispatch_request(
            &state,
            BridgeRequest::Estimate {
                req_id: "r1".into(),
                secret: "0".repeat(64),
                payload: serde_json::json!({ "estimate": {} }),
            },
        );
        assert!(!resp.ok);
        assert!(resp.error.unwrap().contains("secret invalide"));
    }

    #[test]
    fn estimate_with_valid_secret_records_event() {
        let (_tmp, state) = fresh_state();
        // Pair to get a secret.
        let dto = crate::logic::regenerate_pairing_code(&state).unwrap();
        let pair_resp = dispatch_request(
            &state,
            BridgeRequest::Pair {
                req_id: "r-pair".into(),
                code: dto.code,
            },
        );
        let secret = pair_resp.secret.unwrap();

        // Now Estimate.
        let resp = dispatch_request(
            &state,
            BridgeRequest::Estimate {
                req_id: "r-est".into(),
                secret,
                payload: serde_json::json!({
                    "estimate": {
                        "method": "afnor_sobria",
                        "modelId": "gpt-4o-mini",
                        "tokensIn": 50,
                        "tokensOut": 200,
                        "gco2eq": 0.42,
                        "waterMl": 1.8,
                        "energyWh": 0.12
                    }
                }),
            },
        );
        assert!(resp.ok, "got error: {:?}", resp.error);

        // Checks the persisted event.
        let store = state.extension_store.lock().unwrap();
        let events = store.list_events(10, 0).unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model_id, "gpt-4o-mini");
        assert_eq!(events[0].tokens_in, 50);
    }

    #[test]
    fn revoke_with_valid_secret_revokes_pairing() {
        let (_tmp, state) = fresh_state();
        let dto = crate::logic::regenerate_pairing_code(&state).unwrap();
        let pair_resp = dispatch_request(
            &state,
            BridgeRequest::Pair {
                req_id: "r-pair".into(),
                code: dto.code,
            },
        );
        let secret = pair_resp.secret.unwrap();

        let revoke_resp = dispatch_request(
            &state,
            BridgeRequest::Revoke {
                req_id: "r-rev".into(),
                secret: secret.clone(),
            },
        );
        assert!(revoke_resp.ok, "got error: {:?}", revoke_resp.error);

        // Second revoke with the same secret → must no longer match (revoked).
        let resp2 = dispatch_request(
            &state,
            BridgeRequest::Revoke {
                req_id: "r-rev2".into(),
                secret,
            },
        );
        assert!(!resp2.ok);
    }

    #[test]
    fn estimate_without_estimate_field_returns_error() {
        let (_tmp, state) = fresh_state();
        let dto = crate::logic::regenerate_pairing_code(&state).unwrap();
        let pair_resp = dispatch_request(
            &state,
            BridgeRequest::Pair {
                req_id: "r-pair".into(),
                code: dto.code,
            },
        );
        let secret = pair_resp.secret.unwrap();
        let resp = dispatch_request(
            &state,
            BridgeRequest::Estimate {
                req_id: "r-est".into(),
                secret,
                payload: serde_json::json!({ "other": "thing" }),
            },
        );
        assert!(!resp.ok);
        assert!(resp.error.unwrap().contains("estimate"));
    }
}
