//! `sobria-bridge`: Native Messaging bridge between the browser extension and
//! the Sobr.ia desktop app.
//!
//! The binary (`main.rs`) reads framed requests on stdin, forwards each one to
//! the app's local socket, and handles it offline when the app is unreachable.
//! This library holds the protocol, the forward and the spool, shared with
//! `sobria-app` and the tests.
#![deny(unsafe_code)]

use std::io::{Read, Write};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Native messaging host name, as used by `connectNative` and the manifests.
pub const NATIVE_HOST_NAME: &str = "com.sobria.bridge";

/// Spool size above which it is rotated to `incoming.jsonl.bak` (10 MB).
pub const SPOOL_MAX_BYTES: u64 = 10 * 1024 * 1024;

/// Request sent by the extension, tagged by `type`. Serialized as is to the app.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum BridgeRequest {
    Ping {
        #[serde(rename = "reqId")]
        req_id: String,
    },
    Pair {
        #[serde(rename = "reqId")]
        req_id: String,
        code: String,
    },
    Estimate {
        #[serde(rename = "reqId")]
        req_id: String,
        secret: String,
        payload: Value,
    },
    Revoke {
        #[serde(rename = "reqId")]
        req_id: String,
        secret: String,
    },
}

impl BridgeRequest {
    pub fn req_id(&self) -> &str {
        match self {
            BridgeRequest::Ping { req_id }
            | BridgeRequest::Pair { req_id, .. }
            | BridgeRequest::Estimate { req_id, .. }
            | BridgeRequest::Revoke { req_id, .. } => req_id,
        }
    }
}

/// Reply sent to the extension, either built here or decoded from the app.
#[derive(Debug, Serialize, Deserialize)]
pub struct BridgeResponse {
    #[serde(rename = "reqId")]
    pub req_id: String,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pong: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub secret: Option<String>,
    #[serde(rename = "pairingId", skip_serializing_if = "Option::is_none")]
    pub pairing_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
}

impl BridgeResponse {
    pub fn ok(req_id: impl Into<String>) -> Self {
        Self {
            req_id: req_id.into(),
            ok: true,
            error: None,
            pong: None,
            secret: None,
            pairing_id: None,
            fingerprint: None,
        }
    }

    pub fn err(req_id: impl Into<String>, msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(msg.into()),
            ..Self::ok(req_id)
        }
    }
}

/// Reads one frame (little-endian `u32` length, then JSON) from `r`.
/// Returns `Ok(None)` at the end of the stream, including inside the length.
pub fn read_message<R: Read>(r: &mut R) -> Result<Option<BridgeRequest>> {
    let mut len_buf = [0u8; 4];
    if let Err(e) = r.read_exact(&mut len_buf) {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            return Ok(None);
        }
        return Err(e.into());
    }
    let len = u32::from_le_bytes(len_buf) as usize;
    if len > 1024 * 1024 {
        anyhow::bail!("message length exceeds 1 MB: {len}");
    }
    let mut payload = vec![0u8; len];
    r.read_exact(&mut payload)
        .context("read native messaging payload")?;
    let req: BridgeRequest =
        serde_json::from_slice(&payload).with_context(|| format!("decode JSON ({len} bytes)"))?;
    Ok(Some(req))
}

/// Writes one frame on `w`.
pub fn write_message<W: Write>(w: &mut W, resp: &BridgeResponse) -> Result<()> {
    let bytes = serde_json::to_vec(resp)?;
    let len = u32::try_from(bytes.len()).context("response exceeds u32")?;
    w.write_all(&len.to_le_bytes())?;
    w.write_all(&bytes)?;
    w.flush()?;
    Ok(())
}

/// Spool file drained by the app: `~/.sobria/spool/incoming.jsonl`.
pub fn spool_path() -> Result<std::path::PathBuf> {
    let home = dirs::home_dir().context("home_dir introuvable")?;
    Ok(home.join(".sobria").join("spool").join("incoming.jsonl"))
}

fn append_to_spool(payload: &Value) -> Result<()> {
    append_to_spool_at(&spool_path()?, payload, SPOOL_MAX_BYTES)
}

/// Appends `payload` as one JSON line to `path`, after moving the file to
/// `.jsonl.bak` if it is larger than `max_bytes`.
pub fn append_to_spool_at(path: &std::path::Path, payload: &Value, max_bytes: u64) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Ok(meta) = std::fs::metadata(path) {
        if meta.len() > max_bytes {
            let bak = path.with_extension("jsonl.bak");
            let _ = std::fs::remove_file(&bak);
            std::fs::rename(path, &bak)?;
        }
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    let line = serde_json::to_string(payload)?;
    writeln!(file, "{line}")?;
    Ok(())
}

/// Local channel between the bridge and the app, never a network port:
/// `$XDG_RUNTIME_DIR/sobria-bridge.sock`, else `/tmp/sobria-bridge.sock` on
/// Unix, and `\\.\pipe\sobria-bridge` on Windows.
///
/// Neither is restricted to the current OS user yet (global pipe names, world
/// writable `/tmp`): SOB-107.
pub fn default_socket_path() -> std::path::PathBuf {
    #[cfg(unix)]
    {
        if let Some(rt) = std::env::var_os("XDG_RUNTIME_DIR") {
            return std::path::PathBuf::from(rt).join("sobria-bridge.sock");
        }
        std::path::PathBuf::from("/tmp/sobria-bridge.sock")
    }
    #[cfg(windows)]
    {
        std::path::PathBuf::from(r"\\.\pipe\sobria-bridge")
    }
    #[cfg(not(any(unix, windows)))]
    {
        std::path::PathBuf::from("sobria-bridge.sock")
    }
}

/// Read and write timeout of the forward. Unix only: the Windows pipe has none
/// (SOB-96).
#[cfg(unix)]
const FORWARD_TIMEOUT: std::time::Duration = std::time::Duration::from_millis(2_000);

/// Sends `req` to the app on `path` and reads its reply, both framed like the
/// stdio protocol.
///
/// Fails when the channel does not exist (app not running), on a timeout
/// (Unix, 2 s) or on a malformed reply. The binary then handles the request
/// offline, whatever the failure (SOB-190).
pub fn try_forward_to(path: &std::path::Path, req: &BridgeRequest) -> Result<BridgeResponse> {
    #[cfg(unix)]
    {
        use std::os::unix::net::UnixStream;
        let mut stream =
            UnixStream::connect(path).with_context(|| format!("connect {}", path.display()))?;
        stream.set_read_timeout(Some(FORWARD_TIMEOUT))?;
        stream.set_write_timeout(Some(FORWARD_TIMEOUT))?;
        forward_over(&mut stream, req)
    }
    #[cfg(windows)]
    {
        let path_str = path.to_str().context("pipe path non-UTF-8")?;
        let mut stream = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path_str)
            .with_context(|| format!("open pipe {path_str}"))?;
        forward_over(&mut stream, req)
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (path, req);
        anyhow::bail!("plateforme non supportée pour try_forward_to")
    }
}

fn forward_over<S: Read + Write>(stream: &mut S, req: &BridgeRequest) -> Result<BridgeResponse> {
    let bytes = serde_json::to_vec(req).context("encode BridgeRequest")?;
    let len = u32::try_from(bytes.len()).context("request > u32")?;
    stream.write_all(&len.to_le_bytes())?;
    stream.write_all(&bytes)?;
    stream.flush()?;
    let mut len_buf = [0u8; 4];
    stream
        .read_exact(&mut len_buf)
        .context("read response length")?;
    let resp_len = u32::from_le_bytes(len_buf) as usize;
    if resp_len > 1024 * 1024 {
        anyhow::bail!("response too large: {resp_len}");
    }
    let mut payload = vec![0u8; resp_len];
    stream
        .read_exact(&mut payload)
        .context("read response payload")?;
    let resp: BridgeResponse = serde_json::from_slice(&payload).context("decode BridgeResponse")?;
    Ok(resp)
}

/// Keeps the secret out of the spool: 16 hex digits of an unkeyed `×31`
/// polynomial hash, a weak fingerprint the app does not read (SOB-81).
pub fn short_hash(s: &str) -> String {
    let mut acc: u64 = 0;
    for &b in s.as_bytes() {
        acc = acc.wrapping_mul(31).wrapping_add(u64::from(b));
    }
    format!("{:016x}", acc)
}

/// Offline handling, used when the app cannot be reached:
///   - `Ping` → `pong: true` (the bridge is installed);
///   - `Estimate` → appended to the spool, without checking the secret (SOB-119);
///   - `Pair` and `Revoke` → refused, they need the app.
pub fn handle_request(req: BridgeRequest) -> BridgeResponse {
    match req {
        BridgeRequest::Ping { req_id } => BridgeResponse {
            pong: Some(true),
            ..BridgeResponse::ok(req_id)
        },
        BridgeRequest::Pair { req_id, .. } => BridgeResponse::err(
            req_id,
            "Pairing nécessite l'app Sobr.ia desktop ≥ 0.6.0 lancée — \
             non disponible (C27.5.b/c).",
        ),
        BridgeRequest::Estimate {
            req_id,
            secret,
            payload,
        } => {
            let envelope = serde_json::json!({
                "secret_hash": short_hash(&secret),
                "payload": payload,
                "received_at": chrono::Utc::now().to_rfc3339(),
            });
            match append_to_spool(&envelope) {
                Ok(()) => BridgeResponse::ok(req_id),
                Err(e) => BridgeResponse::err(req_id, format!("spool write: {e}")),
            }
        },
        BridgeRequest::Revoke { req_id, .. } => BridgeResponse::err(
            req_id,
            "Revoke nécessite l'app Sobr.ia desktop joignable — \
             effacement local côté extension uniquement.",
        ),
    }
}
