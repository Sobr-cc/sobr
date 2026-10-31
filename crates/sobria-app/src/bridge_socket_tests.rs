//! Characterization of the real bridge socket server (SOB-188).
//!
//! Unlike `bridge_server.rs`, which calls `dispatch_request` directly, these
//! tests start `bridge_server::serve` (the code `run` uses at app start-up)
//! on a private endpoint (a Unix socket in a temp folder, or a uniquely named
//! pipe on Windows) and talk to it as the `sobria-bridge` binary does:
//! one connection per request, a `u32` little-endian length, then the JSON.
//!
//! The server needs an `AppHandle` to find the `AppState`; the tests give it
//! the handle of a mock Tauri app (`tauri::test`).
//!
//! They are tests of the binary (a `#[cfg(test)]` module of `main.rs`), not an
//! integration test under `tests/`: a test executable that really links Tauri
//! needs the Windows manifest that `tauri-build` embeds in the binary target
//! only, and without it the process cannot start (`STATUS_ENTRYPOINT_NOT_FOUND`).

#![allow(clippy::too_many_lines, clippy::similar_names)]

use std::path::{Path, PathBuf};
use std::time::Duration;

use sobria_app::{bridge_server, logic, AppState};
use sobria_bridge::{BridgeRequest, BridgeResponse};
use tauri::test::{mock_app, MockRuntime};
use tauri::{App, Manager};
use tokio::io::{AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tokio::task::JoinHandle;

#[cfg(unix)]
type Client = tokio::net::UnixStream;
#[cfg(windows)]
type Client = tokio::net::windows::named_pipe::NamedPipeClient;

/// Same limit as the server (`MAX_PAYLOAD_BYTES` in `bridge_server.rs`).
const MAX_PAYLOAD_BYTES: u32 = 1024 * 1024;

const NO_PENDING_CODE: &str =
    "aucun code de pairing en attente — génère-en un depuis Sobr.ia /parametres";
const BRIDGE_FINGERPRINT: &str = "sobria-bridge-native-messaging";

// ─── Client side ─────────────────────────────────────────────────────────────

/// A fresh endpoint for one test: no two tests share a socket or a pipe.
fn endpoint(dir: &Path) -> PathBuf {
    #[cfg(unix)]
    {
        dir.join("bridge-test.sock")
    }
    #[cfg(windows)]
    {
        use std::sync::atomic::{AtomicU32, Ordering};
        static NEXT: AtomicU32 = AtomicU32::new(0);
        let _ = dir;
        PathBuf::from(format!(
            r"\\.\pipe\sobria-bridge-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ))
    }
}

/// Connects, retrying while the server is still binding (or, on Windows,
/// while every pipe instance is busy).
async fn connect(endpoint: &Path) -> Client {
    for _ in 0..200 {
        #[cfg(unix)]
        let attempt = tokio::net::UnixStream::connect(endpoint).await;
        #[cfg(windows)]
        let attempt = tokio::net::windows::named_pipe::ClientOptions::new().open(endpoint);
        if let Ok(client) = attempt {
            return client;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    panic!(
        "the bridge server never accepted a connection on {}",
        endpoint.display()
    );
}

async fn write_frame<S: AsyncWrite + Unpin>(stream: &mut S, payload: &[u8]) {
    let len = u32::try_from(payload.len()).unwrap();
    stream.write_all(&len.to_le_bytes()).await.unwrap();
    stream.write_all(payload).await.unwrap();
    stream.flush().await.unwrap();
}

/// Reads one answer; `Err` when the server closes the connection without one.
async fn read_frame<S: AsyncRead + Unpin>(stream: &mut S) -> std::io::Result<Vec<u8>> {
    let mut len = [0u8; 4];
    stream.read_exact(&mut len).await?;
    let mut payload = vec![0u8; u32::from_le_bytes(len) as usize];
    stream.read_exact(&mut payload).await?;
    Ok(payload)
}

/// One request on its own connection, like the bridge binary does.
async fn send(endpoint: &Path, request: &BridgeRequest) -> BridgeResponse {
    let mut client = connect(endpoint).await;
    write_frame(&mut client, &serde_json::to_vec(request).unwrap()).await;
    let payload = read_frame(&mut client).await.expect("the server answers");
    serde_json::from_slice(&payload).expect("the answer is a BridgeResponse")
}

fn ping(req_id: &str) -> BridgeRequest {
    BridgeRequest::Ping {
        req_id: req_id.into(),
    }
}

fn pair(req_id: &str, code: &str) -> BridgeRequest {
    BridgeRequest::Pair {
        req_id: req_id.into(),
        code: code.into(),
    }
}

fn estimate(req_id: &str, secret: &str, payload: serde_json::Value) -> BridgeRequest {
    BridgeRequest::Estimate {
        req_id: req_id.into(),
        secret: secret.into(),
        payload,
    }
}

fn revoke(req_id: &str, secret: &str) -> BridgeRequest {
    BridgeRequest::Revoke {
        req_id: req_id.into(),
        secret: secret.into(),
    }
}

// ─── Server side ─────────────────────────────────────────────────────────────

/// The real server, running against a mock app that manages a fresh `AppState`.
struct Server {
    _dir: tempfile::TempDir,
    app: App<MockRuntime>,
    endpoint: PathBuf,
    task: JoinHandle<anyhow::Result<()>>,
}

impl Server {
    fn start() -> Self {
        let dir = tempfile::tempdir().unwrap();
        let app = mock_app();
        app.manage(AppState::init_in(dir.path()).unwrap());
        let endpoint = endpoint(dir.path());
        let task = Self::spawn(&app, &endpoint);
        Self {
            _dir: dir,
            app,
            endpoint,
            task,
        }
    }

    fn spawn(app: &App<MockRuntime>, endpoint: &Path) -> JoinHandle<anyhow::Result<()>> {
        let handle = app.handle().clone();
        let endpoint = endpoint.to_path_buf();
        tokio::spawn(async move { bridge_server::serve(handle, &endpoint).await })
    }

    fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state::<AppState>()
    }

    async fn send(&self, request: &BridgeRequest) -> BridgeResponse {
        send(&self.endpoint, request).await
    }

    /// A pending pairing code, as the app shows it in its settings.
    fn pending_code(&self) -> String {
        logic::regenerate_pairing_code(self.state().inner())
            .unwrap()
            .code
    }

    /// Pairs and returns the secret.
    async fn paired_secret(&self) -> String {
        let code = self.pending_code();
        let resp = self.send(&pair("r-pair", &code)).await;
        assert!(resp.ok, "pairing refused: {:?}", resp.error);
        resp.secret.expect("a secret")
    }

    /// The server closes a connection that breaks the protocol without an answer.
    async fn assert_no_answer(&self, bytes: &[u8], what: &str) {
        let mut client = connect(&self.endpoint).await;
        client.write_all(bytes).await.unwrap();
        client.flush().await.unwrap();
        assert!(
            read_frame(&mut client).await.is_err(),
            "{what}: expected no answer"
        );
    }
}

impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// A frame (length + payload) as the server expects it.
fn frame(payload: &[u8]) -> Vec<u8> {
    let mut bytes = u32::try_from(payload.len()).unwrap().to_le_bytes().to_vec();
    bytes.extend_from_slice(payload);
    bytes
}

// ─── Tests ───────────────────────────────────────────────────────────────────

#[tokio::test]
async fn ping_gets_a_pong_through_the_socket() {
    let server = Server::start();

    let resp = server.send(&ping("p1")).await;

    assert_eq!(resp.req_id, "p1");
    assert!(resp.ok);
    assert_eq!(resp.pong, Some(true));
    assert!(resp.error.is_none());
    assert!(resp.secret.is_none() && resp.pairing_id.is_none() && resp.fingerprint.is_none());
}

#[tokio::test]
async fn pair_estimate_and_revoke_work_through_the_socket() {
    let server = Server::start();

    let code = server.pending_code();
    let paired = server.send(&pair("r-pair", &code)).await;
    assert!(paired.ok, "pairing refused: {:?}", paired.error);
    assert_eq!(paired.req_id, "r-pair");
    let secret = paired.secret.clone().expect("a secret");
    assert_eq!(secret.len(), 64);
    let pairing_id = paired.pairing_id.clone().expect("a pairing id");
    assert_eq!(paired.fingerprint.as_deref(), Some(BRIDGE_FINGERPRINT));

    let payload = serde_json::json!({
        "estimate": {
            "method": "afnor_sobria", "modelId": "gpt-4o-mini", "tokensIn": 50,
            "tokensOut": 200, "gco2eq": 0.42, "waterMl": 1.8, "energyWh": 0.12
        }
    });
    let stored = server.send(&estimate("r-est", &secret, payload)).await;
    assert!(stored.ok, "estimate refused: {:?}", stored.error);
    assert_eq!(stored.pairing_id.as_deref(), Some(pairing_id.as_str()));
    {
        let events = server
            .state()
            .extension_store
            .lock()
            .unwrap()
            .list_events(10, 0)
            .unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].model_id, "gpt-4o-mini");
        assert_eq!(events[0].tokens_in, 50);
        assert_eq!(events[0].tokens_out, 200);
    }

    let revoked = server.send(&revoke("r-rev", &secret)).await;
    assert!(revoked.ok, "revoke refused: {:?}", revoked.error);
    assert_eq!(revoked.pairing_id.as_deref(), Some(pairing_id.as_str()));

    // A revoked secret is refused by both requests.
    let again = server.send(&revoke("r-rev2", &secret)).await;
    assert!(!again.ok);
    assert_eq!(
        again.error.as_deref(),
        Some("secret invalide — déjà révoqué ?")
    );
    let late = server
        .send(&estimate(
            "r-late",
            &secret,
            serde_json::json!({"estimate": {}}),
        ))
        .await;
    assert!(!late.ok);
    assert_eq!(
        late.error.as_deref(),
        Some("secret invalide ou pairing révoqué")
    );
    assert_eq!(
        server
            .state()
            .extension_store
            .lock()
            .unwrap()
            .list_events(10, 0)
            .unwrap()
            .len(),
        1
    );
}

#[tokio::test]
async fn refused_requests_get_their_pinned_answers_through_the_socket() {
    let server = Server::start();

    // No pending code at all.
    let resp = server.send(&pair("r1", "123456")).await;
    assert!(!resp.ok);
    assert_eq!(resp.req_id, "r1");
    assert_eq!(resp.error.as_deref(), Some(NO_PENDING_CODE));
    assert!(resp.secret.is_none() && resp.pairing_id.is_none() && resp.pong.is_none());

    // A pending code: malformed, then wrong. Refusals keep the code pending.
    let code = server.pending_code();
    let wrong = format!("{:06}", (code.parse::<u32>().unwrap() + 1) % 1_000_000);
    assert_eq!(
        server.send(&pair("r2", "abc")).await.error.as_deref(),
        Some("code à 6 chiffres requis")
    );
    for attempt in 0..3 {
        assert_eq!(
            server.send(&pair("r3", &wrong)).await.error.as_deref(),
            Some("code invalide ou expiré"),
            "attempt {attempt}"
        );
    }
    assert!(server
        .state()
        .extension_store
        .lock()
        .unwrap()
        .list_pairings()
        .unwrap()
        .is_empty());

    // The right code still pairs, once.
    let paired = server.send(&pair("r4", &code)).await;
    assert!(paired.ok, "pairing refused: {:?}", paired.error);
    assert_eq!(
        server.send(&pair("r5", &code)).await.error.as_deref(),
        Some(NO_PENDING_CODE)
    );

    // Estimate and revoke with a secret nobody paired with.
    let unknown = "0".repeat(64);
    let resp = server
        .send(&estimate(
            "r6",
            &unknown,
            serde_json::json!({"estimate": {}}),
        ))
        .await;
    assert_eq!(
        resp.error.as_deref(),
        Some("secret invalide ou pairing révoqué")
    );
    assert_eq!(
        server.send(&revoke("r7", &unknown)).await.error.as_deref(),
        Some("secret invalide — déjà révoqué ?")
    );

    // A valid secret, but a payload without `estimate`.
    let secret = paired.secret.unwrap();
    let resp = server
        .send(&estimate("r8", &secret, serde_json::json!({"other": 1})))
        .await;
    assert!(!resp.ok);
    assert_eq!(resp.error.as_deref(), Some("payload sans champ `estimate`"));
    assert!(server
        .state()
        .extension_store
        .lock()
        .unwrap()
        .list_events(10, 0)
        .unwrap()
        .is_empty());
}

#[tokio::test]
async fn a_connection_that_breaks_the_protocol_gets_no_answer_and_the_server_keeps_serving() {
    let server = Server::start();

    // A length above the limit: the server refuses it after reading the 4 bytes.
    server
        .assert_no_answer(
            &(MAX_PAYLOAD_BYTES + 1).to_le_bytes(),
            "length over the limit",
        )
        .await;
    assert_eq!(
        server.send(&ping("after-oversized")).await.req_id,
        "after-oversized"
    );

    // A frame that is not JSON, then JSON that is not a request.
    server
        .assert_no_answer(&frame(b"not json"), "payload that is not JSON")
        .await;
    server
        .assert_no_answer(
            &frame(br#"{"type":"unknown","reqId":"x"}"#),
            "unknown request type",
        )
        .await;
    assert_eq!(
        server.send(&ping("after-garbage")).await.req_id,
        "after-garbage"
    );

    // A frame cut short, and a client that connects and leaves.
    let mut truncated = 100u32.to_le_bytes().to_vec();
    truncated.extend_from_slice(b"short");
    let mut client = connect(&server.endpoint).await;
    client.write_all(&truncated).await.unwrap();
    drop(client);
    drop(connect(&server.endpoint).await);
    assert_eq!(
        server.send(&ping("after-hangups")).await.req_id,
        "after-hangups"
    );
}

#[tokio::test]
async fn several_clients_are_served_one_after_the_other_and_at_once() {
    let server = Server::start();

    for n in 0..5 {
        let id = format!("seq-{n}");
        assert_eq!(server.send(&ping(&id)).await.req_id, id);
    }

    let (ra, rb, rc) = (ping("par-a"), ping("par-b"), ping("par-c"));
    let (a, b, c) = tokio::join!(server.send(&ra), server.send(&rb), server.send(&rc));
    assert_eq!(
        (a.req_id.as_str(), b.req_id.as_str(), c.req_id.as_str()),
        ("par-a", "par-b", "par-c")
    );
    assert!(a.ok && b.ok && c.ok);
}

#[tokio::test]
async fn the_secret_of_one_pairing_is_checked_against_the_stored_hash() {
    // A secret that is the right length but was never issued is refused,
    // while the issued one is accepted: the server does not just check the shape.
    let server = Server::start();
    let secret = server.paired_secret().await;
    let mut forged = secret.clone();
    forged.replace_range(0..1, if secret.starts_with('0') { "1" } else { "0" });

    let resp = server
        .send(&estimate(
            "r1",
            &forged,
            serde_json::json!({"estimate": {}}),
        ))
        .await;
    assert_eq!(
        resp.error.as_deref(),
        Some("secret invalide ou pairing révoqué")
    );
    let resp = server
        .send(&estimate(
            "r2",
            &secret,
            serde_json::json!({"estimate": {}}),
        ))
        .await;
    assert!(resp.ok, "estimate refused: {:?}", resp.error);
}

/// Unix: a socket file left by a crash does not prevent the next start.
#[cfg(unix)]
#[tokio::test]
async fn a_stale_socket_file_is_replaced_at_start() {
    let dir = tempfile::tempdir().unwrap();
    let app = mock_app();
    app.manage(AppState::init_in(dir.path()).unwrap());
    let endpoint = endpoint(dir.path());
    std::fs::write(&endpoint, b"left by a crash").unwrap();

    let task = Server::spawn(&app, &endpoint);

    assert_eq!(send(&endpoint, &ping("p1")).await.req_id, "p1");
    task.abort();
}

/// Windows: the pipe is created with `first_pipe_instance`, so a second
/// server on the same name fails to start instead of sharing the pipe.
#[cfg(windows)]
#[tokio::test]
async fn a_second_server_on_the_same_pipe_fails_to_start() {
    let server = Server::start();
    assert!(server.send(&ping("p1")).await.ok); // the first server is up

    let second = Server::spawn(&server.app, &server.endpoint);
    let result = tokio::time::timeout(Duration::from_secs(5), second)
        .await
        .expect("the second server returns")
        .expect("the second server does not panic");

    let error = result.expect_err("the second server cannot create the pipe");
    assert!(
        error.to_string().contains("create named pipe"),
        "got: {error:#}"
    );
    assert!(
        server.send(&ping("p2")).await.ok,
        "the first server is unaffected"
    );
}
